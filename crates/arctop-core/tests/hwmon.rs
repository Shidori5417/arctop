//! hwmon entegrasyon testleri: a750 fixture'ı, t0/t1 örnekleri (§8).

use std::time::{Duration, Instant};

use arctop_core::source::hwmon::HwmonSource;
use arctop_core::{Device, Driver, Metric, MetricSource, Snapshot, SysFs};

/// Testte kullanılan a750 cihazı (fixture: `card1`, i915).
fn a750() -> Device {
    Device {
        card: 1,
        pci_slot: "0000:03:00.0".to_string(),
        device_id: 0x56a1,
        name: "Intel Arc A750".to_string(),
        driver: Driver::I915,
    }
}

/// a750 fixture kökü (`t0` ya da `t1`).
fn fixture(t: &str) -> SysFs {
    SysFs::with_root(format!(
        "{}/../../tests/fixtures/a750/{t}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn t0_sonrasi_t1_guc_farktan_hesaplanir() {
    let dev = a750();
    let mut src = HwmonSource::new();

    // probe: beş alan da, kartta verilen adlarla ve okunabilir durumda.
    let caps = src.probe(&fixture("t0"), &dev);
    let names: Vec<_> = caps.fields.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        [
            "hwmon.power",
            "hwmon.power_limit",
            "hwmon.temp",
            "hwmon.fan",
            "hwmon.voltage",
        ]
    );
    for (name, m) in &caps.fields {
        assert!(m.is_available(), "{name} okunabilir olmalı");
    }

    // İlk örnek: Δ yok, güç "ilk örnek"; diğer alanlar dolu.
    let t0 = Instant::now();
    let mut snap = Snapshot::empty(t0);
    src.sample(&fixture("t0"), &dev, t0, &mut snap);
    assert_eq!(snap.power_w, Metric::Unavailable("ilk örnek"));
    assert_eq!(snap.power_limit_w, Metric::Value(190.0));
    assert_eq!(snap.temp_c, Metric::Value(49.0));
    assert_eq!(snap.fan_rpm, Metric::Value(0)); // 0 geçerli: fan durmuş
    assert_eq!(snap.voltage_mv, Metric::Value(825));
    // sample yalnızca kendi beş alanına dokunur.
    assert_eq!(snap.freq.act, Metric::Unavailable("örneklenmedi"));
    assert_eq!(snap.vram_used, Metric::Unavailable("örneklenmedi"));

    // İkinci örnek (t1 kökü, +2 s): W = ΔµJ / 1e6 / Δs ≈ 53,2 W.
    let t1 = t0 + Duration::from_secs(2);
    let mut snap = Snapshot::empty(t1);
    src.sample(&fixture("t1"), &dev, t1, &mut snap);
    let expected = (469_796_980_224u64 - 469_690_564_208) as f32 / 1e6 / 2.0;
    match snap.power_w {
        Metric::Value(w) => assert!(
            (w - expected).abs() < 0.1,
            "güç {w} W, beklenen ~{expected} W"
        ),
        other => panic!("güç değer olmalı: {other:?}"),
    }
    assert_eq!(snap.power_limit_w, Metric::Value(190.0));
    assert_eq!(snap.temp_c, Metric::Value(49.0));
    assert_eq!(snap.fan_rpm, Metric::Value(0));
    assert_eq!(snap.voltage_mv, Metric::Value(755));
}

#[test]
fn sayac_geriye_gidince_sifirlandi_der_ve_referans_yenilenir() {
    let dev = a750();
    let mut src = HwmonSource::new();
    src.probe(&fixture("t0"), &dev);

    let t0 = Instant::now();
    let mut snap = Snapshot::empty(t0);
    src.sample(&fixture("t0"), &dev, t0, &mut snap); // "ilk örnek"

    let t1 = t0 + Duration::from_secs(2);
    let mut snap = Snapshot::empty(t1);
    src.sample(&fixture("t1"), &dev, t1, &mut snap);
    assert!(snap.power_w.is_available());

    // t1'den sonra t0: sayaç geriye gider (taşma/sıfırlanma).
    let t2 = t1 + Duration::from_secs(2);
    let mut snap = Snapshot::empty(t2);
    src.sample(&fixture("t0"), &dev, t2, &mut snap);
    assert_eq!(snap.power_w, Metric::Unavailable("sayaç sıfırlandı"));

    // Referans güncellendiği için bir sonraki örnek yeniden değer verir.
    let t3 = t2 + Duration::from_secs(2);
    let mut snap = Snapshot::empty(t3);
    src.sample(&fixture("t1"), &dev, t3, &mut snap);
    assert!(
        snap.power_w.is_available(),
        "sıfırlamadan sonra referans yenilenmeli: {:?}",
        snap.power_w
    );
}

#[test]
fn hwmon_bulunamazsa_tum_alanlar_hwmon_yok() {
    let mut dev = a750();
    dev.card = 9; // fixture'da card9 yok
    let mut src = HwmonSource::new();

    let caps = src.probe(&fixture("t0"), &dev);
    assert_eq!(caps.fields.len(), 5);
    for (name, m) in &caps.fields {
        assert_eq!(m, &Metric::Unavailable("hwmon yok"), "{name}");
    }

    let mut snap = Snapshot::empty(Instant::now());
    src.sample(&fixture("t0"), &dev, Instant::now(), &mut snap);
    assert_eq!(snap.power_w, Metric::Unavailable("hwmon yok"));
    assert_eq!(snap.power_limit_w, Metric::Unavailable("hwmon yok"));
    assert_eq!(snap.temp_c, Metric::Unavailable("hwmon yok"));
    assert_eq!(snap.fan_rpm, Metric::Unavailable("hwmon yok"));
    assert_eq!(snap.voltage_mv, Metric::Unavailable("hwmon yok"));
}
