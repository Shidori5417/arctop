//! DRM fdinfo metrik kaynağı: süreç bazlı GPU kullanımı (§3).
//!
//! `/proc/<pid>/fdinfo/<fd>` dosyalarından motor doluluğu (toplam ve süreç başına),
//! süreç listesi ve kullanılan VRAM hesaplanır.
//!
//! # Eşzamanlılık ve iptal
//! Kaynak hiçbir kilit tutmaz ve iş parçacığı oluşturmaz: `MetricSource` sözleşmesi gereği
//! örnekleyici iş parçacığında `&mut self` ile sırayla çağrılır. Bu yüzden kilit sırası
//! sorunu yoktur. İptal (cancellation) yoktur; `sample` senkron çalışır ve yarıda
//! kesilmez. Kaynak düşürülürse yalnızca bellekteki sayaçlar atılır, dış kaynak tutulmaz.
//!
//! # Kart filtresi
//! Yalnızca `drm-pdev` değeri `dev.pci_slot` ile eşleşen istemciler sayılır.
//! `dev.pci_slot` boşsa (fixture'lar) filtre uygulanmaz.
//!
//! # VRAM
//! `vram_used`, tüm istemcilerin `drm-resident-local0` toplamıdır. Süreçler arasında
//! paylaşılan tamponlar birden çok istemcide görünebileceğinden çift sayılabilir.
//! `vram_total` için i915'te sysfs girdisi yoktur (SYSFS-NOTES); bu kaynak onu `Unavailable("sysfs'de yok")` olarak işaretler.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::model::{
    Capabilities, Device, Driver, EngineClass, EngineUsage, Metric, ProcUsage, Snapshot,
};
use crate::source::MetricSource;
use crate::sysfs::SysFs;

/// Sabit çıktı sırası: `Render, Copy, Video, VideoEnhance, Compute`.
const CLASSES: [EngineClass; 5] = [
    EngineClass::Render,
    EngineClass::Copy,
    EngineClass::Video,
    EngineClass::VideoEnhance,
    EngineClass::Compute,
];
const N: usize = CLASSES.len();

/// fdinfo'daki sınıf adından `CLASSES` dizinine.
fn class_index(name: &str) -> Option<usize> {
    CLASSES.iter().position(|c| c.as_fdinfo_key() == name)
}

/// Ayrıştırılmış tek bir fdinfo dosyası (satırlara ödünç referans verir).
#[derive(Debug, Clone, PartialEq)]
struct FdInfo<'a> {
    driver: Driver,
    client_id: u64,
    pdev: Option<&'a str>,
    /// i915: `drm-engine-<sınıf>` (ns).
    busy_ns: [Option<u64>; N],
    /// i915: `drm-engine-capacity-<sınıf>` (varsayılan 1).
    capacity: [u64; N],
    /// xe: `drm-cycles-<sınıf>`.
    cycles: [Option<u64>; N],
    /// xe: `drm-total-cycles-<sınıf>`.
    total_cycles: [Option<u64>; N],
    /// `drm-resident-local0` (bayt).
    resident_local0: u64,
}

/// `<sayı>[ <birim>]` değerini (sayı, birim) olarak ayırır. Birim yoksa boş döner.
fn split_value(v: &str) -> Option<(u64, &str)> {
    let v = v.trim();
    let (num, unit) = match v.split_once(char::is_whitespace) {
        Some((n, u)) => (n, u.trim()),
        None => (v, ""),
    };
    Some((num.parse().ok()?, unit))
}

/// Bellek değeri → bayt. Birimsiz = bayt; `KiB`/`MiB`/`GiB` desteklenir.
fn parse_memory(v: &str) -> Option<u64> {
    let (n, unit) = split_value(v)?;
    let shift = match unit {
        "" => 0,
        "KiB" => 10,
        "MiB" => 20,
        "GiB" => 30,
        _ => return None,
    };
    n.checked_mul(1u64 << shift)
}

/// Motor süresi: yalnızca `ns` birimli değer.
fn parse_ns(v: &str) -> Option<u64> {
    match split_value(v)? {
        (n, "ns") => Some(n),
        _ => None,
    }
}

/// Birimsiz sayaç (kapasite, çevrim).
fn parse_plain(v: &str) -> Option<u64> {
    match split_value(v)? {
        (n, "") => Some(n),
        _ => None,
    }
}

/// Bir fdinfo dosyasını ayrıştırır. Sürücü `i915`/`xe` değilse ya da `drm-client-id`
/// yoksa `None`. Bozuk satırlar sessizce atlanır. Saf fonksiyon, G/Ç yapmaz.
fn parse_fdinfo(text: &str) -> Option<FdInfo<'_>> {
    let mut driver = None;
    let mut client_id = None;
    let mut info = FdInfo {
        driver: Driver::I915,
        client_id: 0,
        pdev: None,
        busy_ns: [None; N],
        capacity: [1; N],
        cycles: [None; N],
        total_cycles: [None; N],
        resident_local0: 0,
    };
    for line in text.lines() {
        let Some((key, val)) = line.split_once(':') else {
            continue;
        };
        let (key, val) = (key.trim(), val.trim());
        let Some(key) = key.strip_prefix("drm-") else {
            continue;
        };
        match key {
            "driver" => {
                driver = match val {
                    "i915" => Some(Driver::I915),
                    "xe" => Some(Driver::Xe),
                    _ => return None,
                }
            }
            "client-id" => client_id = val.parse().ok(),
            "pdev" => info.pdev = Some(val),
            "resident-local0" => info.resident_local0 = parse_memory(val).unwrap_or(0),
            _ => {
                // Önce daha uzun önekler: `engine-capacity-` ve `total-cycles-`.
                if let Some(c) = key.strip_prefix("engine-capacity-") {
                    if let (Some(i), Some(n)) = (class_index(c), parse_plain(val)) {
                        info.capacity[i] = n;
                    }
                } else if let Some(c) = key.strip_prefix("engine-") {
                    if let (Some(i), Some(n)) = (class_index(c), parse_ns(val)) {
                        info.busy_ns[i] = Some(n);
                    }
                } else if let Some(c) = key.strip_prefix("total-cycles-") {
                    if let (Some(i), Some(n)) = (class_index(c), parse_plain(val)) {
                        info.total_cycles[i] = Some(n);
                    }
                } else if let Some(c) = key.strip_prefix("cycles-")
                    && let (Some(i), Some(n)) = (class_index(c), parse_plain(val))
                {
                    info.cycles[i] = Some(n);
                }
            }
        }
    }
    info.driver = driver?;
    info.client_id = client_id?;
    Some(info)
}

/// Bir istemcinin önceki örnekteki sayaçları.
#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    busy_ns: [Option<u64>; N],
    cycles: [Option<u64>; N],
    total_cycles: [Option<u64>; N],
}

impl Counters {
    fn of(i: &FdInfo<'_>) -> Self {
        Self {
            busy_ns: i.busy_ns,
            cycles: i.cycles,
            total_cycles: i.total_cycles,
        }
    }

    /// `prev`'e göre sınıf `i` doluluğu (% , `[0,100]`). Hesaplanamıyorsa `None`.
    fn pct(&self, prev: &Counters, info: &FdInfo<'_>, i: usize, wall_ns: f64) -> Option<f64> {
        let pct = match info.driver {
            Driver::I915 => {
                let d = self.busy_ns[i]?.checked_sub(prev.busy_ns[i]?)?;
                if wall_ns <= 0.0 {
                    return None;
                }
                d as f64 / (wall_ns * info.capacity[i].max(1) as f64) * 100.0
            }
            Driver::Xe => {
                let dc = self.cycles[i]?.checked_sub(prev.cycles[i]?)?;
                let dt = self.total_cycles[i]?.checked_sub(prev.total_cycles[i]?)?;
                if dt == 0 {
                    return None;
                }
                dc as f64 / dt as f64 * 100.0
            }
        };
        Some(pct.clamp(0.0, 100.0))
    }
}

/// fdinfo kaynağı: motor doluluğu, süreçler, VRAM kullanımı.
#[derive(Debug, Default)]
pub struct FdinfoSource {
    /// Önceki örnekteki sayaçlar; anahtar `(pid, drm-client-id)`.
    prev: HashMap<(u32, u64), Counters>,
    /// Bu örnekte görülenler; örnek sonunda `prev` ile yer değiştirir (kaybolanlar temizlenir).
    next: HashMap<(u32, u64), Counters>,
    /// Önceki örneğin zamanı.
    prev_at: Option<Instant>,
    /// Cihazda en az bir kez görülen motor sınıfları.
    seen: [bool; N],
}

impl FdinfoSource {
    /// Boş durumlu yeni kaynak.
    pub fn new() -> Self {
        Self::default()
    }

    /// Süreçteki aday fdinfo yollarını `buf`'a koyar. `false`: süreç atlanmalı.
    ///
    /// `/proc/<pid>/fd` okunabiliyorsa yalnızca `renderD*`/`card*` hedefli fd'ler alınır;
    /// dizin yoksa (fixture) tüm `fdinfo/*` taranır; başka hata (izin yok, süreç bitti) → atla.
    fn candidate_paths(fs: &SysFs, pid: u32, buf: &mut Vec<PathBuf>) -> bool {
        buf.clear();
        match fs.read_dir(format!("/proc/{pid}/fd")) {
            Ok(fds) => {
                for fd in fds {
                    let Ok(target) = fs.read_link_name(&fd) else {
                        continue;
                    };
                    if (target.starts_with("renderD") || target.starts_with("card"))
                        && let Some(name) = fd.file_name()
                    {
                        buf.push(
                            Path::new("/proc")
                                .join(pid.to_string())
                                .join("fdinfo")
                                .join(name),
                        );
                    }
                }
                true
            }
            Err(crate::ReadError::NotFound(_)) => {
                match fs.read_dir(format!("/proc/{pid}/fdinfo")) {
                    Ok(all) => {
                        buf.extend(all);
                        true
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    #[cfg(test)]
    fn tracked_clients(&self) -> usize {
        self.prev.len()
    }
}

impl MetricSource for FdinfoSource {
    fn name(&self) -> &'static str {
        "fdinfo"
    }

    fn probe(&mut self, fs: &SysFs, _dev: &Device) -> Capabilities {
        let m: Metric<()> = fs.read_dir("/proc").map(|_| ()).into();
        Capabilities {
            fields: vec![
                ("fdinfo.engines", m.clone()),
                ("fdinfo.procs", m.clone()),
                ("fdinfo.vram_used", m),
                ("fdinfo.vram_total", Metric::Unavailable("sysfs'de yok")),
            ],
        }
    }

    fn sample(&mut self, fs: &SysFs, dev: &Device, now: Instant, out: &mut Snapshot) {
        let entries = match fs.read_dir("/proc") {
            Ok(e) => e,
            Err(e) => {
                out.vram_used = Metric::Unavailable(e.reason());
                return;
            }
        };
        let wall_ns = self
            .prev_at
            .and_then(|p| now.checked_duration_since(p))
            .map(|d| d.as_nanos() as f64);
        self.next.clear();

        let mut tot_pct = [0.0f64; N];
        let mut tot_vram = 0u64;
        let mut procs = Vec::new();
        let mut paths = Vec::new();
        let mut clients: Vec<u64> = Vec::new();

        for entry in entries {
            let Some(pid) = entry
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            if !Self::candidate_paths(fs, pid, &mut paths) {
                continue;
            }
            clients.clear();
            let mut pid_pct = [0.0f64; N];
            let mut pid_has = [false; N];
            let mut pid_vram = 0u64;
            let mut found = false;

            for path in &paths {
                // Erişilemeyen ya da biten fd sessizce atlanır.
                let Ok(text) = fs.read_string(path) else {
                    continue;
                };
                let Some(info) = parse_fdinfo(&text) else {
                    continue;
                };
                if !dev.pci_slot.is_empty() && info.pdev != Some(dev.pci_slot.as_str()) {
                    continue;
                }
                // Aynı süreçte aynı istemciyi tutan fd'ler tek sayılır.
                if clients.contains(&info.client_id) {
                    continue;
                }
                clients.push(info.client_id);
                found = true;
                pid_vram = pid_vram.saturating_add(info.resident_local0);

                let cur = Counters::of(&info);
                let key = (pid, info.client_id);
                for i in 0..N {
                    if cur.busy_ns[i].is_some() || cur.cycles[i].is_some() {
                        self.seen[i] = true;
                    }
                }
                // İlk görülen istemci (ya da ilk örnek) için doluluk raporlanmaz.
                if let (Some(prev), Some(w)) = (self.prev.get(&key), wall_ns) {
                    for i in 0..N {
                        if let Some(p) = cur.pct(prev, &info, i, w) {
                            pid_pct[i] += p;
                            pid_has[i] = true;
                        }
                    }
                }
                self.next.insert(key, cur);
            }

            if !found {
                continue;
            }
            tot_vram = tot_vram.saturating_add(pid_vram);
            let mut engines = Vec::new();
            for i in 0..N {
                if pid_has[i] {
                    let p = pid_pct[i].clamp(0.0, 100.0);
                    engines.push(EngineUsage {
                        class: CLASSES[i],
                        busy_pct: p as f32,
                    });
                    tot_pct[i] += pid_pct[i];
                }
            }
            let comm = fs
                .read_string(format!("/proc/{pid}/comm"))
                .unwrap_or_else(|_| "?".to_string());
            procs.push(ProcUsage {
                pid,
                comm,
                engines,
                vram_bytes: Metric::Value(pid_vram),
            });
        }

        // Kaybolan istemciler: yalnızca bu örnekte görülenler `prev` olur.
        std::mem::swap(&mut self.prev, &mut self.next);
        self.prev_at = Some(now);

        procs.sort_by_key(|p| p.pid);
        out.procs = procs;
        out.vram_used = Metric::Value(tot_vram);
        out.vram_total = Metric::Unavailable("sysfs'de yok");
        out.engines = if wall_ns.is_some() {
            (0..N)
                .filter(|&i| self.seen[i])
                .map(|i| EngineUsage {
                    class: CLASSES[i],
                    busy_pct: tot_pct[i].clamp(0.0, 100.0) as f32,
                })
                .collect()
        } else {
            // İlk örnek: Δ yok; 0 raporlamak yerine liste boş.
            Vec::new()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn memory_units() {
        assert_eq!(parse_memory("25088 KiB"), Some(25088 * 1024));
        assert_eq!(parse_memory("8 MiB"), Some(8 << 20));
        assert_eq!(parse_memory("2 GiB"), Some(2 << 30));
        assert_eq!(parse_memory("0"), Some(0));
        assert_eq!(parse_memory("512"), Some(512));
        assert_eq!(parse_memory("5 furlong"), None);
        assert_eq!(parse_memory("abc KiB"), None);
        assert_eq!(parse_memory(""), None);
    }

    #[test]
    fn ns_and_plain() {
        assert_eq!(parse_ns("123 ns"), Some(123));
        assert_eq!(parse_ns("0 ns"), Some(0));
        assert_eq!(parse_ns("123"), None);
        assert_eq!(parse_plain("2"), Some(2));
        assert_eq!(parse_plain("2 ns"), None);
    }

    #[test]
    fn parse_basic_i915() {
        let t = "drm-driver:\ti915\ndrm-client-id:\t22\ndrm-pdev:\t0000:03:00.0\n\
                 drm-resident-local0:\t25088 KiB\ndrm-engine-video:\t123 ns\n\
                 drm-engine-capacity-video:\t2\ngarbage line\ndrm-engine-render:\tbad ns\n\
                 drm-engine-video-enhance:\t7 ns\n";
        let i = parse_fdinfo(t).unwrap();
        assert_eq!(i.driver, Driver::I915);
        assert_eq!(i.client_id, 22);
        assert_eq!(i.pdev, Some("0000:03:00.0"));
        assert_eq!(i.resident_local0, 25088 * 1024);
        assert_eq!(i.busy_ns[2], Some(123));
        assert_eq!(i.capacity[2], 2);
        assert_eq!(i.busy_ns[0], None); // bozuk satır
        assert_eq!(i.busy_ns[3], Some(7));
        assert_eq!(i.capacity[3], 1);
    }

    #[test]
    fn parse_xe_cycles() {
        let t = "drm-driver:\txe\ndrm-client-id:\t3\ndrm-cycles-rcs:\t5\n\
                 drm-cycles-render:\t50\ndrm-total-cycles-render:\t200\n";
        let i = parse_fdinfo(t).unwrap();
        assert_eq!(i.driver, Driver::Xe);
        assert_eq!(i.cycles[0], Some(50));
        assert_eq!(i.total_cycles[0], Some(200));
    }

    #[test]
    fn parse_rejects_other_driver_or_missing_id() {
        assert!(parse_fdinfo("drm-driver:\tamdgpu\ndrm-client-id:\t1\n").is_none());
        assert!(parse_fdinfo("drm-driver:\ti915\n").is_none());
        assert!(parse_fdinfo("pos:\t0\nflags:\t02\n").is_none());
    }

    #[test]
    fn xe_percentage() {
        let mut a = FdInfo {
            driver: Driver::Xe,
            client_id: 1,
            pdev: None,
            busy_ns: [None; N],
            capacity: [1; N],
            cycles: [None; N],
            total_cycles: [None; N],
            resident_local0: 0,
        };
        let mut p = Counters::of(&a);
        p.cycles[0] = Some(100);
        p.total_cycles[0] = Some(1000);
        a.cycles[0] = Some(400);
        a.total_cycles[0] = Some(2000);
        let c = Counters::of(&a);
        assert_eq!(c.pct(&p, &a, 0, 1e9), Some(30.0));
        // Δtotal = 0 → hesaplanamaz.
        assert_eq!(p.pct(&p, &a, 0, 1e9), None);
    }

    fn fixture(rel: &str) -> SysFs {
        SysFs::with_root(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(rel),
        )
    }

    fn dev() -> Device {
        Device {
            card: 1,
            pci_slot: "0000:00:02.0".into(),
            device_id: 0,
            name: "t".into(),
            driver: Driver::I915,
        }
    }

    #[test]
    fn vanished_clients_are_dropped_from_state() {
        let mut s = FdinfoSource::new();
        let t = Instant::now();
        let mut out = Snapshot::empty(t);
        s.sample(&fixture("synthetic/t0"), &dev(), t, &mut out);
        // (100,1) (200,7) (400,4); (300) başka kart, (500) amdgpu.
        assert_eq!(s.tracked_clients(), 3);
        s.sample(
            &fixture("synthetic/t1"),
            &dev(),
            t + Duration::from_secs(1),
            &mut out,
        );
        // 400 kayboldu, 600 geldi.
        assert_eq!(s.tracked_clients(), 3);
        assert!(!s.prev.contains_key(&(400, 4)));
        assert!(s.prev.contains_key(&(600, 9)));
    }
}
