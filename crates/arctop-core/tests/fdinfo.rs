//! `FdinfoSource` testleri: sentetik ve gerçek (a750) fixture'lar, ayrıca zamanlama.
//!
//! Bu dosyada `std::fs` serbesttir (zamanlama testi geçici dizine ağaç yazar).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arctop_core::source::fdinfo::FdinfoSource;
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

fn write_tree(root: &Path, n: u32, busy: u64) {
    for pid in 1000..1000 + n {
        let dir = root.join(format!("proc/{pid}/fdinfo"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join(format!("proc/{pid}/comm")), "bench\n").unwrap();
        let body = format!(
            "drm-driver:\ti915\ndrm-client-id:\t{pid}\ndrm-pdev:\t0000:00:02.0\n\
             drm-resident-local0:\t1024 KiB\ndrm-engine-render:\t{busy} ns\n\
             drm-engine-video:\t{busy} ns\ndrm-engine-capacity-video:\t2\n"
        );
        std::fs::write(dir.join("3"), body).unwrap();
    }
}

/// `cargo test --release -- --ignored` ile çalıştırılır (SPEC: 500 süreçte < 5 ms).
#[test]
#[ignore]
fn sample_500_procs_under_5ms() {
    let root = std::env::temp_dir().join(format!("arctop-fdinfo-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write_tree(&root, 500, 1_000_000);
    let fs = SysFs::with_root(&root);
    let d = dev("0000:00:02.0");
    let mut src = FdinfoSource::new();
    let t = Instant::now();
    let mut out = Snapshot::empty(t);
    src.sample(&fs, &d, t, &mut out);
    write_tree(&root, 500, 2_000_000);
    // Kararlı durum: ilk örnek (sayaç ekleme) ölçüm dışı.
    let start = Instant::now();
    src.sample(&fs, &d, t + Duration::from_secs(1), &mut out);
    let el = start.elapsed();
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(out.procs.len(), 500);
    assert!(el < Duration::from_millis(5), "örnekleme {el:?}");
}
