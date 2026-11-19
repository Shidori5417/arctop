//! Frekans kaynağı entegrasyon testleri (§8): a750/t0 fixture, olmayan kök, xe yolu.

use std::time::Instant;

use arctop_core::source::MetricSource;
use arctop_core::source::freq::FreqSource;
use arctop_core::{Device, Driver, Metric, Snapshot, SysFs, discover::discover};

/// a750 makinesinin t0 anındaki fixture kökü.
fn a750_t0() -> SysFs {
    SysFs::with_root(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/a750/t0"
    ))
}

/// Var olmayan fixture kökü (bilinçli olarak oluşturulmaz).
fn yok() -> SysFs {
    SysFs::with_root(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/yok"
    ))
}

/// Elle kurulmış cihaz (fixture gerektirmeyen testler için).
fn device(card: u32, driver: Driver) -> Device {
    Device {
        card,
        pci_slot: String::new(),
        device_id: 0x56a1,
        name: "test".into(),
        driver,
    }
}

#[test]
fn a750_t0_da_frekanslar_okunur() {
    let fs = a750_t0();
    let devices = discover(&fs).expect("keşif hata döndürdü");
    let dev = devices.first().expect("fixture'da bir kart olmalı");
    assert_eq!(dev.driver, Driver::I915);

    let mut src = FreqSource::new();
    assert_eq!(src.name(), "freq");

    // probe: a750'de beş alan da okunabilir ve adlar sabit sıradadır.
    let caps = src.probe(&fs, dev);
    let names: Vec<&str> = caps.fields.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["freq.act", "freq.cur", "freq.min", "freq.max", "freq.boost"]
    );
    for (name, m) in &caps.fields {
        assert!(matches!(m, Metric::Value(())), "okunamayan alan: {name}");
    }

    let mut snap = Snapshot::empty(Instant::now());
    src.sample(&fs, dev, Instant::now(), &mut snap);
    assert_eq!(snap.freq.max, Metric::Value(2400));
    assert_eq!(snap.freq.min, Metric::Value(300));
    assert_eq!(snap.freq.boost, Metric::Value(2400));
    // act ve cur canlı değerlerdir; kart yalnız Value olmalarını şart koşar.
    assert!(matches!(snap.freq.act, Metric::Value(_)));
    assert!(matches!(snap.freq.cur, Metric::Value(_)));
    // Kaynak freq dışındaki alanlara dokunmaz.
    assert_eq!(snap.power_w, Metric::Unavailable("örneklenmedi"));
}

#[test]
fn olmayan_kokte_tum_alanlar_unavailable() {
    let fs = yok();
    let dev = device(0, Driver::I915);
    let mut src = FreqSource::new();

    let caps = src.probe(&fs, &dev);
    assert_eq!(caps.fields.len(), 5);
    for (name, m) in caps.fields {
        assert_eq!(m, Metric::Unavailable("yok"), "alan: {name}");
    }

    let mut snap = Snapshot::empty(Instant::now());
    src.sample(&fs, &dev, Instant::now(), &mut snap);
    assert_eq!(snap.freq.act, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.cur, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.min, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.max, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.boost, Metric::Unavailable("yok"));
}

#[test]
fn xe_alanlari_unavailable_boost_xede_yok() {
    // Fixture'sız: gerçek sysfs'te var olmayan kart numarası, testi gerçek
    // xe donanımı olan makinede de deterministik kılar (yol hiç bulunamaz).
    let fs = SysFs::real();
    let dev = device(9999, Driver::Xe);
    let mut src = FreqSource::new();

    let caps = src.probe(&fs, &dev);
    assert_eq!(caps.fields.len(), 5);
    for (name, m) in caps.fields {
        let expected = if name == "freq.boost" {
            Metric::Unavailable("xe'de yok")
        } else {
            Metric::Unavailable("yok")
        };
        assert_eq!(m, expected, "alan: {name}");
    }

    let mut snap = Snapshot::empty(Instant::now());
    src.sample(&fs, &dev, Instant::now(), &mut snap);
    assert_eq!(snap.freq.boost, Metric::Unavailable("xe'de yok"));
    assert_eq!(snap.freq.act, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.cur, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.min, Metric::Unavailable("yok"));
    assert_eq!(snap.freq.max, Metric::Unavailable("yok"));
}
