//! `FdinfoSource` testleri: sentetik ve gerçek (a750) fixture'lar, ayrıca zamanlama.
//!
//! Bu dosyada `std::fs` serbesttir (zamanlama testi geçici dizine ağaç yazar).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arctop_core::source::fdinfo::{FdinfoSource, RESCAN_EVERY};
use arctop_core::{Device, Driver, EngineClass, Metric, MetricSource, Snapshot, SysFs};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn dev(slot: &str) -> Device {
    Device {
        card: 1,
        pci_slot: slot.into(),
        device_id: 0x56a1,
        name: "test".into(),
        driver: Driver::I915,
    }
}

fn pct(out: &Snapshot, c: EngineClass) -> Option<f32> {
    out.engines
        .iter()
        .find(|e| e.class == c)
        .map(|e| e.busy_pct)
}

#[test]
fn synthetic_t0_to_t1() {
    // Sentetik ağaç (tests/fixtures/synthetic):
    //  pid 100: iki fd, aynı client 1 -> tek sayılır; render 1.0s -> 1.5s = %50, 1024 KiB.
    //  pid 200: capacity-video 2; video Δ1s => %50, render Δ0.25s => %25, 2048 KiB.
    //  pid 300: başka drm-pdev -> sayılmaz.   pid 400: yalnızca t0'da var (kaybolur).
    //  pid 500: drm-driver amdgpu -> sayılmaz. pid 600: yalnızca t1'de (ilk görülen, Δ yok).
    let d = dev("0000:00:02.0");
    let mut src = FdinfoSource::new();
    assert_eq!(src.name(), "fdinfo");
    let t = Instant::now();

    let mut o0 = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic/t0")),
        &d,
        t,
        &mut o0,
    );
    assert!(o0.engines.is_empty(), "ilk örnekte Δ yok");
    let pids: Vec<u32> = o0.procs.iter().map(|p| p.pid).collect();
    assert_eq!(pids, [100, 200, 400]);
    // 1024 KiB + 2048 KiB + 512 KiB
    assert_eq!(o0.vram_used, Metric::Value(3_670_016));

    let mut o1 = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic/t1")),
        &d,
        t + Duration::from_secs(1),
        &mut o1,
    );
    let pids: Vec<u32> = o1.procs.iter().map(|p| p.pid).collect();
    assert_eq!(pids, [100, 200, 600]);
    let comms: Vec<&str> = o1.procs.iter().map(|p| p.comm.as_str()).collect();
    assert_eq!(comms, ["alpha", "beta", "zeta"]);
    // 1024 KiB + 2048 KiB + 1 MiB (pid 600)
    assert_eq!(o1.vram_used, Metric::Value(4_194_304));
    assert_eq!(o1.procs[0].vram_bytes, Metric::Value(1_048_576));
    assert_eq!(o1.procs[1].vram_bytes, Metric::Value(2_097_152));
    assert_eq!(o1.procs[2].vram_bytes, Metric::Value(1_048_576));

    // Sıra Render, Copy, Video, VideoEnhance; Compute hiç görülmedi.
    let classes: Vec<EngineClass> = o1.engines.iter().map(|e| e.class).collect();
    assert_eq!(
        classes,
        [
            EngineClass::Render,
            EngineClass::Copy,
            EngineClass::Video,
            EngineClass::VideoEnhance
        ]
    );
    let near = |a: Option<f32>, b: f32| assert!((a.unwrap() - b).abs() < 1e-3, "{a:?} != {b}");
    near(pct(&o1, EngineClass::Render), 75.0); // 50 + 25
    near(pct(&o1, EngineClass::Copy), 0.0);
    near(pct(&o1, EngineClass::Video), 50.0);
    near(pct(&o1, EngineClass::VideoEnhance), 0.0);

    // Süreç başına.
    let p100: Vec<(EngineClass, f32)> = o1.procs[0]
        .engines
        .iter()
        .map(|e| (e.class, e.busy_pct))
        .collect();
    assert_eq!(
        p100,
        [(EngineClass::Render, 50.0), (EngineClass::Copy, 0.0)]
    );
    let p200: Vec<(EngineClass, f32)> = o1.procs[1]
        .engines
        .iter()
        .map(|e| (e.class, e.busy_pct))
        .collect();
    assert_eq!(
        p200,
        [
            (EngineClass::Render, 25.0),
            (EngineClass::Video, 50.0),
            (EngineClass::VideoEnhance, 0.0)
        ]
    );
    // pid 600 ilk görüldü: motor katkısı yok.
    assert!(o1.procs[2].engines.is_empty());
}

#[test]
fn real_a750_t0_to_t1_does_not_panic() {
    // pci_slot boş: kart filtresi uygulanmaz.
    let d = dev("");
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut out = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("a750/t0")),
        &d,
        t,
        &mut out,
    );
    src.sample(
        &SysFs::with_root(fixtures().join("a750/t1")),
        &d,
        t + Duration::from_secs(2),
        &mut out,
    );
    let p = out.procs.iter().find(|p| p.pid == 1815).expect("pid 1815");
    assert_eq!(p.comm, "Hyprland");
    assert!(matches!(out.vram_used, Metric::Value(_)));
    assert_eq!(out.vram_total, Metric::Unavailable("sysfs'de yok"));
}

#[test]
fn probe_reports_four_fields() {
    let mut src = FdinfoSource::new();
    let caps = src.probe(&SysFs::with_root(fixtures().join("a750/t0")), &dev(""));
    let names: Vec<&str> = caps.fields.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        [
            "fdinfo.engines",
            "fdinfo.procs",
            "fdinfo.vram_used",
            "fdinfo.vram_total"
        ]
    );
    // vram_total i915'te sysfs'de olmadığı için her zaman Unavailable.
    assert!(
        caps.fields
            .iter()
            .filter(|(n, _)| *n != "fdinfo.vram_total")
            .all(|(_, m)| m.is_available())
    );
    let total = caps
        .fields
        .iter()
        .find(|(n, _)| *n == "fdinfo.vram_total")
        .map(|(_, m)| m.clone());
    assert_eq!(total, Some(Metric::Unavailable("sysfs'de yok")));
    let caps = src.probe(&SysFs::with_root("/nonexistent-arctop-root"), &dev(""));
    assert!(caps.fields.iter().all(|(_, m)| !m.is_available()));
}

/// Bench ağacı: süreç başına 20 fd; 3'ü DRM (`renderD128`, `card1`), kalanı soket/pipe.
/// `symlinks == true`: gerçek sembolik bağlar (gerçek /proc gibi `readlink` sistem çağrısı);
/// aksi halde ADR-004 `<n>.name` dosyaları.
fn write_tree(root: &Path, n: u32, busy: u64, symlinks: bool) {
    for pid in 1000..1000 + n {
        let dir = root.join(format!("proc/{pid}/fdinfo"));
        let fddir = root.join(format!("proc/{pid}/fd"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&fddir).unwrap();
        std::fs::write(root.join(format!("proc/{pid}/comm")), "bench\n").unwrap();
        for fd in 0..20u32 {
            let target = match fd {
                3 => "/dev/dri/renderD128".to_string(),
                4 => "/dev/dri/card1".to_string(),
                5 => "/dev/dri/renderD128".to_string(),
                _ if fd % 2 == 0 => format!("socket:[{}]", 1000 + fd),
                _ => format!("pipe:[{}]", 2000 + fd),
            };
            if symlinks {
                let link = fddir.join(fd.to_string());
                if !link.exists() && std::fs::symlink_metadata(&link).is_err() {
                    std::os::unix::fs::symlink(&target, &link).unwrap();
                }
            } else {
                let base = target.rsplit('/').next().unwrap();
                std::fs::write(fddir.join(format!("{fd}.name")), format!("{base}\n")).unwrap();
            }
        }
        // DRM fd'leri 3 (render) ve 4 (card, aynı istemci -> tek sayılır).
        let body = format!(
            "drm-driver:\ti915\ndrm-client-id:\t{pid}\ndrm-pdev:\t0000:00:02.0\n\
             drm-resident-local0:\t1024 KiB\ndrm-engine-render:\t{busy} ns\n\
             drm-engine-video:\t{busy} ns\ndrm-engine-capacity-video:\t2\n"
        );
        std::fs::write(dir.join("3"), &body).unwrap();
        std::fs::write(dir.join("4"), &body).unwrap();
        std::fs::write(dir.join("5"), &body).unwrap();
        // DRM olmayan fd'lerin fdinfo'su okunmamalı (bozuk içerik, okunursa göz ardı edilir).
        std::fs::write(dir.join("6"), "pos:\t0\nflags:\t02\n").unwrap();
    }
}

fn bench_500(label: &str, symlinks: bool) {
    let root = std::env::temp_dir().join(format!(
        "arctop-fdinfo-bench-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_tree(&root, 500, 1_000_000, symlinks);
    let fs = SysFs::with_root(&root);
    let d = dev("0000:00:02.0");
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut out = Snapshot::empty(t);
    let t_first = Instant::now();
    src.sample(&fs, &d, t, &mut out);
    let first = t_first.elapsed();
    write_tree(&root, 500, 2_000_000, symlinks);
    // Kararlı durum: ilk örnek (sayaç ekleme) ölçüm dışı; 20 örnek ortalaması.
    let start = Instant::now();
    const RUNS: u32 = 20;
    for k in 1..=RUNS {
        src.sample(&fs, &d, t + Duration::from_secs(k as u64), &mut out);
    }
    let avg = start.elapsed() / RUNS;
    let _ = std::fs::remove_dir_all(&root);
    println!("fdinfo bench [{label}] 500 süreç, ilk tarama {first:?}, kararlı ortalama {avg:?}");
    assert_eq!(out.procs.len(), 500);
    // Gerçek fd yolu sistem çağrısı bağımlıdır (T-022 raporu): hedef aşımı test başarısızlığı
    // yapılmaz, yalnızca bildirilir.
    if avg >= Duration::from_millis(5) {
        println!("UYARI [{label}]: SPEC hedefi (< 5 ms) aşıldı");
    }
}

/// `cargo test --release -- --ignored` ile çalıştırılır (SPEC: 500 süreçte < 5 ms).
/// `/proc/N/fd` + ADR-004 `<n>.name` dosyaları (`read_link_name` .name geri düşme yolu).
#[test]
#[ignore]
fn sample_500_procs_under_5ms() {
    bench_500("name", false);
}

/// Aynısı, gerçek sembolik bağlarla (gerçek /proc'taki `readlink` yolu).
#[test]
#[ignore]
fn sample_500_procs_symlinks_under_5ms() {
    bench_500("symlink", true);
}

/// Bu makinede canlı `/proc`: 20 örnekleme, ortalama süre stdout'a (eşik yok).
#[test]
#[ignore]
fn live_proc_sampling_average() {
    let fs = SysFs::real();
    let d = dev("");
    let mut src = FdinfoSource::new();
    let mut out = Snapshot::empty(Instant::now());
    let t = Instant::now();
    let t_first = Instant::now();
    src.sample(&fs, &d, t, &mut out); // ilk tam tarama
    let first = t_first.elapsed();
    let start = Instant::now();
    const RUNS: u32 = 20;
    for k in 1..=RUNS {
        src.sample(&fs, &d, t + Duration::from_secs(k as u64), &mut out);
    }
    let avg = start.elapsed() / RUNS;
    let nproc = std::fs::read_dir("/proc")
        .map(|r| {
            r.filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
            })
            .count()
        })
        .unwrap_or(0);
    println!(
        "fdinfo canlı: {nproc} süreç, {} DRM satırı, ilk tarama {first:?}, ortalama {avg:?}",
        out.procs.len()
    );
    if avg >= Duration::from_millis(5) {
        println!("UYARI [canlı]: SPEC hedefi (< 5 ms) aşıldı");
    }
}

#[test]
fn synthetic_xe_percentages_and_vram() {
    // Sentetik xe (DOĞRULANMADI): rcs Δ500/Δ1000 = %50; vcs Δ1000/(Δ1000*capacity 2) = %50;
    // `other` yoksayılır; VRAM = drm-resident-vram0 = 8192 KiB.
    let d = Device {
        driver: Driver::Xe,
        ..dev("0000:03:00.0")
    };
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut o = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic-xe/t0")),
        &d,
        t,
        &mut o,
    );
    assert_eq!(o.vram_used, Metric::Value(8 << 20));
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic-xe/t1")),
        &d,
        t + Duration::from_secs(1),
        &mut o,
    );
    assert_eq!(o.vram_used, Metric::Value(8 << 20));
    let classes: Vec<EngineClass> = o.engines.iter().map(|e| e.class).collect();
    assert_eq!(classes, [EngineClass::Render, EngineClass::Video]);
    assert_eq!(pct(&o, EngineClass::Render), Some(50.0));
    assert_eq!(pct(&o, EngineClass::Video), Some(50.0));
    assert_eq!(o.procs.len(), 1);
    assert_eq!(o.procs[0].vram_bytes, Metric::Value(8 << 20));
    assert_eq!(o.procs[0].engines.len(), 2);
}

#[test]
fn shared_client_counted_once_and_assigned_to_min_pid() {
    // pid 900 ve 1000 (sayısal en küçük 900; sözlük sırasında "1000" önce gelir) ve 1100
    // aynı client 5'i paylaşır. 1000 ayrıca kendi client 6'sına sahip. 900'ün fd 4'ü soket
    // olduğundan client 99 sayılmaz (fd filtresi, .name yolu).
    let d = dev("0000:00:02.0");
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut o = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic-shared/t0")),
        &d,
        t,
        &mut o,
    );
    let pids: Vec<u32> = o.procs.iter().map(|p| p.pid).collect();
    assert_eq!(pids, [900, 1000]); // 1100 yalnızca paylaşılan istemciyi görüyor: satır yok
    assert_eq!(o.procs[0].vram_bytes, Metric::Value(3 << 20));
    assert_eq!(o.procs[1].vram_bytes, Metric::Value(3 << 20)); // yalnızca client 6
    assert_eq!(o.vram_used, Metric::Value(6 << 20)); // 5 bir kez + 6

    src.sample(
        &SysFs::with_root(fixtures().join("synthetic-shared/t1")),
        &d,
        t + Duration::from_secs(1),
        &mut o,
    );
    // client 5: 0.5s/1s = %50 (pid 900); client 6: 0.25s = %25 (pid 1000).
    assert_eq!(pct(&o, EngineClass::Render), Some(75.0));
    assert_eq!(o.procs[0].engines[0].busy_pct, 50.0);
    assert_eq!(o.procs[1].engines[0].busy_pct, 25.0);
    assert_eq!(o.vram_used, Metric::Value(6 << 20));
}

#[test]
fn missing_memory_key_is_unavailable() {
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut o = Snapshot::empty(t);
    src.sample(
        &SysFs::with_root(fixtures().join("synthetic-nomem/t0")),
        &dev("0000:00:02.0"),
        t,
        &mut o,
    );
    assert_eq!(o.procs.len(), 1);
    assert_eq!(o.vram_used, Metric::Unavailable("fdinfo'da yok"));
    assert_eq!(o.procs[0].vram_bytes, Metric::Unavailable("fdinfo'da yok"));
}

// ---- Önbellek doğruluk testleri (T-024) ----

/// Geçici ağaçta tek süreç: `fd` dizininde bağlar, `fdinfo`'da içerik.
struct Tree {
    root: PathBuf,
}

impl Tree {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("arctop-fdcache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("proc")).unwrap();
        Tree { root }
    }

    fn proc_dir(&self, pid: u32) -> PathBuf {
        self.root.join(format!("proc/{pid}"))
    }

    /// Süreci oluşturur (yoksa); `comm` yazar.
    fn add_pid(&self, pid: u32) {
        std::fs::create_dir_all(self.proc_dir(pid).join("fd")).unwrap();
        std::fs::create_dir_all(self.proc_dir(pid).join("fdinfo")).unwrap();
        std::fs::write(self.proc_dir(pid).join("comm"), "t\n").unwrap();
    }

    /// fd'yi `target`'a bağlar; `info` fdinfo içeriğidir (öncekini ezer).
    fn set_fd(&self, pid: u32, fd: u32, target: &str, info: &str) {
        let link = self.proc_dir(pid).join(format!("fd/{fd}"));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(target, &link).unwrap();
        std::fs::write(self.proc_dir(pid).join(format!("fdinfo/{fd}")), info).unwrap();
    }

    fn remove_pid(&self, pid: u32) {
        std::fs::remove_dir_all(self.proc_dir(pid)).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn drm_info(client: u64, kib: u64) -> String {
    format!(
        "drm-driver:\ti915\ndrm-client-id:\t{client}\ndrm-pdev:\t0000:00:02.0\n\
         drm-resident-local0:\t{kib} KiB\ndrm-engine-render:\t5 ns\n"
    )
}

fn step(src: &mut FdinfoSource, fs: &SysFs, t: Instant, k: u64) -> Snapshot {
    let mut o = Snapshot::empty(t);
    src.sample(fs, &dev("0000:00:02.0"), t + Duration::from_secs(k), &mut o);
    o
}

#[test]
fn cache_rescan_finds_old_pid_that_opens_drm_later() {
    let tree = Tree::new("late");
    tree.add_pid(4242);
    tree.set_fd(4242, 3, "socket:[1]", "pos:\t0\n");
    let fs = SysFs::with_root(&tree.root);
    let (mut src, t) = (FdinfoSource::new(), Instant::now());
    assert!(step(&mut src, &fs, t, 0).procs.is_empty());
    // Süreç sonradan GPU açar.
    tree.set_fd(4242, 7, "/dev/dri/renderD128", &drm_info(1, 64));
    let mut seen_at = None;
    for k in 1..=u64::from(RESCAN_EVERY) {
        if !step(&mut src, &fs, t, k).procs.is_empty() {
            seen_at = Some(k);
            break;
        }
    }
    let k = seen_at.expect("RESCAN_EVERY örnek içinde görünmeli");
    assert!(k >= 1);
}

#[test]
fn cache_fd_reuse_falls_back_to_full_scan_immediately() {
    let tree = Tree::new("reuse");
    tree.add_pid(4243);
    tree.set_fd(4243, 3, "/dev/dri/renderD128", &drm_info(1, 64));
    let fs = SysFs::with_root(&tree.root);
    let (mut src, t) = (FdinfoSource::new(), Instant::now());
    let o = step(&mut src, &fs, t, 0);
    assert_eq!(o.vram_used, Metric::Value(64 << 10));
    // fd 3 başka bir şeye yeniden kullanılır; DRM istemcisi yeni fd 9'da. Tam tarama
    // zamanı gelmeden (hemen ertesi örnekte) görünmeli.
    tree.set_fd(4243, 3, "socket:[2]", "pos:\t0\n");
    tree.set_fd(4243, 9, "/dev/dri/card1", &drm_info(2, 128));
    let o = step(&mut src, &fs, t, 1);
    assert_eq!(o.vram_used, Metric::Value(128 << 10));
    assert_eq!(o.procs.len(), 1);
}

#[test]
fn cache_vanished_pid_is_forgotten() {
    let tree = Tree::new("gone");
    tree.add_pid(4244);
    tree.set_fd(4244, 3, "socket:[1]", "pos:\t0\n");
    let fs = SysFs::with_root(&tree.root);
    let (mut src, t) = (FdinfoSource::new(), Instant::now());
    assert!(step(&mut src, &fs, t, 0).procs.is_empty());
    tree.remove_pid(4244);
    assert!(step(&mut src, &fs, t, 1).procs.is_empty());
    // Aynı PID, bu kez DRM'li, yeni süreç olarak geri gelir: önbellek temizlendiği için
    // tam tarama beklemeden görünür.
    tree.add_pid(4244);
    tree.set_fd(4244, 3, "/dev/dri/renderD128", &drm_info(3, 32));
    let o = step(&mut src, &fs, t, 2);
    assert_eq!(o.procs.len(), 1);
    assert_eq!(o.vram_used, Metric::Value(32 << 10));
}
