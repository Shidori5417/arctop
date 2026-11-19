//! Keşif entegrasyon testi: `tests/fixtures/a750/t0` sysfs/proc kopyası (§8).

use arctop_core::{Driver, SysFs, discover::discover};

/// a750 makinesinin t0 anındaki fixture kökü.
fn a750_t0() -> SysFs {
    SysFs::with_root(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/a750/t0"
    ))
}

#[test]
fn fixture_tam_bir_a750_bulur() {
    let devices = discover(&a750_t0()).expect("keşif hata döndürdü");

    assert_eq!(
        devices.len(),
        1,
        "fixture'da tam bir kart olmalı: {devices:?}"
    );
    let dev = &devices[0];
    assert_eq!(dev.card, 1);
    assert_eq!(dev.device_id, 0x56a1);
    assert_eq!(dev.driver, Driver::I915);
    assert_eq!(dev.name, "Intel Arc A750");
    // pci_slot bilinçli olarak kontrol edilmez: fixture'da `device` bir
    // dizin olduğundan bağ yoktur ve alan boş kalır (bkz. discover doc'u).
}
