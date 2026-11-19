//! `SysFs` fixture (a750/t0) testleri.

use std::path::{Path, PathBuf};

use arctop_core::{ReadError, SysFs};

fn fs() -> SysFs {
    SysFs::with_root(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/a750/t0"))
}

#[test]
fn read_u64_ok() {
    assert_eq!(
        fs().read_u64("/sys/class/drm/card1/gt_max_freq_mhz")
            .unwrap(),
        2400
    );
}

#[test]
fn read_u64_parse_error() {
    // vendor "0x8086" biçimindedir; düz u64 olarak ayrıştırılamaz.
    let e = fs()
        .read_u64("/sys/class/drm/card1/device/vendor")
        .unwrap_err();
    assert!(matches!(e, ReadError::Parse { .. }), "{e:?}");
}

#[test]
fn read_string_trims_newline() {
    let s = fs()
        .read_string("/sys/class/drm/card1/device/vendor")
        .unwrap();
    assert_eq!(s, "0x8086");
}

#[test]
fn read_link_name_falls_back_to_name_file() {
    let n = fs()
        .read_link_name("/sys/class/drm/card1/device/driver")
        .unwrap();
    assert_eq!(n, "i915");
}

#[test]
fn missing_is_not_found() {
    let e = fs()
        .read_u64("/sys/class/drm/card1/yok_boyle_bir_dosya")
        .unwrap_err();
    assert!(matches!(e, ReadError::NotFound(_)), "{e:?}");
    let e = fs().read_link_name("/sys/class/drm/card1/yok").unwrap_err();
    assert!(matches!(e, ReadError::NotFound(_)), "{e:?}");
}

#[test]
fn read_dir_sorted_rootless() {
    let v = fs().read_dir("/sys/class/drm/card1").unwrap();
    assert!(v.contains(&PathBuf::from("/sys/class/drm/card1/gt_max_freq_mhz")));
    assert!(v.iter().all(|p| p.starts_with("/sys/class/drm/card1")));
    let mut sorted = v.clone();
    sorted.sort();
    assert_eq!(v, sorted);
}
