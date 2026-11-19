//! hwmon metrik kaynağı: güç, güç limiti, sıcaklık, fan ve voltaj okumaları (§3).
//!
//! Güç, mümkünse doğrudan `powerK_input` (µW) dosyasından; yoksa kümülatif
//! `energyK_input` (µJ) sayacının ardışık iki örnek arasındaki farkından
//! hesaplanır. Birden çok girdi olduğunda sabit indeks varsayılmaz: seçim
//! `*_label` dosyalarına göre yapılır (bkz. [`select_index`]).

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::model::{Capabilities, Device, Metric, Snapshot};
use crate::source::MetricSource;
use crate::sysfs::SysFs;

/// hwmon kaynağı: güç, limit, sıcaklık, fan, voltaj.
#[derive(Debug, Default)]
pub struct HwmonSource {
    /// `probe`'da bulunan hwmon dizini (ör. `/sys/class/drm/card1/device/hwmon/hwmon2`).
    dir: Option<PathBuf>,
    /// Doğrudan güç girdisinin indeksi (`powerK_input`, µW); yoksa `None`.
    power_input: Option<u32>,
    /// Enerji sayacının indeksi (`energyK_input`, µJ); yoksa `None`.
    energy_input: Option<u32>,
    /// Sıcaklık girdisinin indeksi (`tempK_input`, m°C); yoksa `None`.
    temp_input: Option<u32>,
    /// Önceki enerji örneği: (zaman, sayaç µJ). Güç Δ hesabının referansı.
    last_energy: Option<(Instant, u64)>,
}

impl HwmonSource {
    /// Yeni kaynak örneği; hwmon dizini `probe` ile bulunana kadar tüm alanlar
    /// `Unavailable("hwmon yok")` olur.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enerji sayacının farkından gücü hesaplar ve `last_energy` referansını
    /// günceller. Sayaç okunamazsa referans temizlenir (bir sonraki örnek
    /// yeniden "ilk örnek" olur).
    fn energy_power(&mut self, fs: &SysFs, dir: &Path, now: Instant) -> Metric<f32> {
        let Some(k) = self.energy_input else {
            return Metric::Unavailable("yok");
        };
        let path = dir.join(format!("energy{k}_input"));
        let uj = match fs.read_u64(&path) {
            Ok(uj) => uj,
            Err(e) => {
                self.last_energy = None;
                return Metric::Unavailable(e.reason());
            }
        };
        let power = match self.last_energy {
            // İlk örnek: karşılaştırılacak önceki değer yok.
            None => Metric::Unavailable("ilk örnek"),
            // Sayaç geriye gitti (taşma ya da sıfırlanma).
            Some((_, prev)) if uj < prev => Metric::Unavailable("sayaç sıfırlandı"),
            Some((at, prev)) => match now.checked_duration_since(at) {
                // Δt > 0: W = ΔµJ / 1e6 / Δs.
                Some(dt) if !dt.is_zero() => {
                    Metric::Value(((uj - prev) as f64 / 1e6 / dt.as_secs_f64()) as f32)
                }
                // Δt = 0: güç tanımsız.
                _ => Metric::Unavailable("ilk örnek"),
            },
        };
        self.last_energy = Some((now, uj));
        power
    }
}

impl MetricSource for HwmonSource {
    fn name(&self) -> &'static str {
        "hwmon"
    }

    fn probe(&mut self, fs: &SysFs, dev: &Device) -> Capabilities {
        // Yeniden probe (ör. kart değişimi) eski durumu tamamen sıfırlar.
        *self = Self::default();
        let base = PathBuf::from(format!("/sys/class/drm/card{}/device/hwmon", dev.card));
        let dir = find_hwmon_dir(fs, &base);
        let fields = match dir.as_deref() {
            None => vec![
                ("hwmon.power", Metric::Unavailable("hwmon yok")),
                ("hwmon.power_limit", Metric::Unavailable("hwmon yok")),
                ("hwmon.temp", Metric::Unavailable("hwmon yok")),
                ("hwmon.fan", Metric::Unavailable("hwmon yok")),
                ("hwmon.voltage", Metric::Unavailable("hwmon yok")),
            ],
            Some(d) => {
                self.power_input = select_index(&scan_inputs(fs, d, "power"), "card");
                self.energy_input = select_index(&scan_inputs(fs, d, "energy"), "card");
                self.temp_input = select_index(&scan_inputs(fs, d, "temp"), "pkg");
                let power = self.power_input.is_some() || self.energy_input.is_some();
                let limit = read_power_limit(fs, d).is_available();
                let temp = self.temp_input.is_some();
                let fan = fs.read_u64(d.join("fan1_input")).is_ok();
                let voltage = fs.read_u64(d.join("in0_input")).is_ok();
                vec![
                    ("hwmon.power", avail(power)),
                    ("hwmon.power_limit", avail(limit)),
                    ("hwmon.temp", avail(temp)),
                    ("hwmon.fan", avail(fan)),
                    ("hwmon.voltage", avail(voltage)),
                ]
            }
        };
        self.dir = dir;
        Capabilities { fields }
    }

    fn sample(&mut self, fs: &SysFs, _dev: &Device, now: Instant, out: &mut Snapshot) {
        let Some(dir) = self.dir.clone() else {
            const YOK: &str = "hwmon yok";
            out.power_w = Metric::Unavailable(YOK);
            out.power_limit_w = Metric::Unavailable(YOK);
            out.temp_c = Metric::Unavailable(YOK);
            out.fan_rpm = Metric::Unavailable(YOK);
            out.voltage_mv = Metric::Unavailable(YOK);
            return;
        };
        // Güç: doğrudan girdi varsa onu kullan; yoksa enerji sayacı farkından.
        out.power_w = match self.power_input {
            Some(k) => read_uw(fs, &dir, &format!("power{k}_input")),
            None => self.energy_power(fs, &dir, now),
        };
        out.power_limit_w = read_power_limit(fs, &dir);
        out.temp_c = match self.temp_input {
            Some(k) => fs
                .read_u64(dir.join(format!("temp{k}_input")))
                .map(|mc| mc as f32 / 1000.0)
                .into(),
            None => Metric::Unavailable("yok"),
        };
        // `0` geçerli bir değerdir: fan durmuş / gerilim ölçülmüyor olabilir.
        out.fan_rpm = fs.read_u64(dir.join("fan1_input")).map(|v| v as u32).into();
        out.voltage_mv = fs.read_u64(dir.join("in0_input")).map(|v| v as u32).into();
    }
}

/// `dir` altındaki hwmon dizinlerinden `name` dosyası `i915` ya da `xe` olan
/// ilkini döndürür (girdiler sıralı geldiği için alfabetik ilk eşleşme);
/// uygun dizin yoksa `None`.
fn find_hwmon_dir(fs: &SysFs, base: &Path) -> Option<PathBuf> {
    for entry in fs.read_dir(base).ok()? {
        if let Ok(name) = fs.read_string(entry.join("name"))
            && (name == "i915" || name == "xe")
        {
            return Some(entry);
        }
    }
    None
}

/// `dir` içindeki `<prefix>K_input` dosyalarını (K, `<prefix>K_label`
/// içeriği) çiftleri olarak, K'ya göre sıralı döndürür. Label dosyası yoksa
/// ya da okunamazsa `None` olur.
fn scan_inputs(fs: &SysFs, dir: &Path, prefix: &str) -> Vec<(u32, Option<String>)> {
    let mut out = Vec::new();
    for entry in fs.read_dir(dir).unwrap_or_default() {
        let Some(name) = entry.file_name().and_then(std::ffi::OsStr::to_str) else {
            continue;
        };
        let Some(k) = name
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix("_input"))
            .and_then(|k| k.parse::<u32>().ok())
        else {
            continue;
        };
        let label = fs.read_string(dir.join(format!("{prefix}{k}_label"))).ok();
        out.push((k, label));
    }
    out.sort();
    out
}

/// Girdi listesinden okunacak indeksi seçer: tercih edilen label'a uyan en
/// küçük K; uyan girdi yoksa en küçük K. Liste boşsa `None`.
fn select_index(entries: &[(u32, Option<String>)], preferred_label: &str) -> Option<u32> {
    entries
        .iter()
        .filter(|(_, label)| label.as_deref() == Some(preferred_label))
        .map(|(k, _)| *k)
        .min()
        .or_else(|| entries.iter().map(|(k, _)| *k).min())
}

/// `dir/<file>` içeriğini µW olarak okuyup W'a çevirir.
fn read_uw(fs: &SysFs, dir: &Path, file: &str) -> Metric<f32> {
    fs.read_u64(dir.join(file)).map(|uw| uw as f32 / 1e6).into()
}

/// Güç limitini okur: `power1_max` (µW) > 0 ise o; değilse `power1_crit`
/// (> 0); ikisi de yoksa `Unavailable("yok")`. `power1_rated_max` bilinçli
/// olarak kullanılmaz (bazı sistemlerde 0 gelir).
fn read_power_limit(fs: &SysFs, dir: &Path) -> Metric<f32> {
    for file in ["power1_max", "power1_crit"] {
        if let Ok(uw) = fs.read_u64(dir.join(file))
            && uw > 0
        {
            return Metric::Value(uw as f32 / 1e6);
        }
    }
    Metric::Unavailable("yok")
}

/// Yetenek girdisi: okunabilirse `Value(())`, değilse `Unavailable("yok")`.
fn avail(ok: bool) -> Metric<()> {
    if ok {
        Metric::Value(())
    } else {
        Metric::Unavailable("yok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tercih edilen label, en küçük K'den önceliklidir.
    #[test]
    fn tercih_edilen_label_secilir() {
        let entries = vec![(1, Some("pkg".into())), (2, Some("card".into()))];
        assert_eq!(select_index(&entries, "card"), Some(2));
        assert_eq!(select_index(&entries, "pkg"), Some(1));
    }

    /// Uyan label yoksa (label dosyası olmayan girdilerde bile) en küçük K.
    #[test]
    fn uyan_label_yoksa_en_kucuk_k() {
        let entries = vec![(3, None), (1, None), (5, Some("x".into()))];
        assert_eq!(select_index(&entries, "card"), Some(1));
        assert_eq!(select_index(&entries, "pkg"), Some(1));
    }

    /// Boş listede seçim yoktur.
    #[test]
    fn bos_liste_secim_yoktur() {
        assert_eq!(select_index(&[], "card"), None);
    }
}
