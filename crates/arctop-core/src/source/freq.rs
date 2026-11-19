//! GT frekans metrik kaynağı (§3): i915 `gt_*_freq_mhz`, xe `freq0/*_freq`.
//!
//! i915 yolları a750 donanımında doğrulanmıştır (SYSFS-NOTES "Frekans (i915)"
//! satırı). xe yolları doğrulanmamıştır; yolun üstündeki yorumlara bakın.

use std::time::Instant;

use crate::model::{Capabilities, Device, Driver, FreqInfo, Metric, Snapshot};
use crate::source::MetricSource;
use crate::sysfs::SysFs;

/// GT frekans kaynağı (MHz).
#[derive(Debug, Default)]
pub struct FreqSource;

impl FreqSource {
    /// Yeni bir frekans kaynağı; iç durum tutmaz (Δ gereksizdir).
    pub fn new() -> Self {
        Self
    }
}

/// Bir frekans alanı: sysfs dosya adı ile `FreqInfo` üyesi birebir eşleşir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    /// Gerçekleşen frekans.
    Act,
    /// İstenen frekans.
    Cur,
    /// Alt sınır.
    Min,
    /// Üst sınır.
    Max,
    /// Boost frekansı; xe'de karşılığı yoktur.
    Boost,
}

impl Field {
    /// `Capabilities.fields` sırasındaki tüm alanlar.
    const ALL: [Field; 5] = [Field::Act, Field::Cur, Field::Min, Field::Max, Field::Boost];

    /// `Capabilities` alan adı (örn. `freq.act`).
    fn cap_name(self) -> &'static str {
        match self {
            Field::Act => "freq.act",
            Field::Cur => "freq.cur",
            Field::Min => "freq.min",
            Field::Max => "freq.max",
            Field::Boost => "freq.boost",
        }
    }

    /// i915 dosya adındaki alan bölümü (`gt_<x>_freq_mhz` içindeki `<x>`).
    fn i915_name(self) -> &'static str {
        match self {
            Field::Act => "act",
            Field::Cur => "cur",
            Field::Min => "min",
            Field::Max => "max",
            Field::Boost => "boost",
        }
    }

    /// xe dosya adındaki alan bölümü (`<x>_freq` içindeki `<x>`); boost yok.
    fn xe_name(self) -> Option<&'static str> {
        match self {
            Field::Act => Some("act"),
            Field::Cur => Some("cur"),
            Field::Min => Some("min"),
            Field::Max => Some("max"),
            Field::Boost => None, // xe'de boost karşılığı yok (ARCHITECTURE §3)
        }
    }
}

/// Alanın sysfs yolu; boost xe'de olmadığından `None` döner.
fn path_for(dev: &Device, field: Field) -> Option<String> {
    match dev.driver {
        // Doğrulandı (i915, a750): düz tamsayı, MHz.
        Driver::I915 => Some(format!(
            "/sys/class/drm/card{}/gt_{}_freq_mhz",
            dev.card,
            field.i915_name()
        )),
        Driver::Xe => field.xe_name().map(|name| {
            // DOĞRULANMADI (xe): ARCHITECTURE §3 — bu yollar donanımda doğrulanmadı.
            format!(
                "/sys/class/drm/card{}/device/tile0/gt0/freq0/{name}_freq",
                dev.card
            )
        }),
    }
}

/// Değer `u32`'ye sığmıyorsa "ayrıştırılamadı" (`Parse` nedeniyle aynı metin).
fn fit_u32(v: u64) -> Metric<u32> {
    match u32::try_from(v) {
        Ok(v) => Metric::Value(v),
        Err(_) => Metric::Unavailable("ayrıştırılamadı"),
    }
}

/// Bir alanı okur: okuma hatası `Metric::from` ile kısa nedene düşer.
fn read_field(fs: &SysFs, dev: &Device, field: Field) -> Metric<u32> {
    let Some(path) = path_for(dev, field) else {
        // Yalnızca xe boost buraya düşer.
        return Metric::Unavailable("xe'de yok");
    };
    match fs.read_u64(path.as_str()) {
        Ok(v) => fit_u32(v),
        Err(e) => Metric::from(Err(e)),
    }
}

/// `Metric<u32>`'den `Metric<()>`'ye: probe çıktısında değer atılır, neden kalır.
fn to_unit(m: Metric<u32>) -> Metric<()> {
    match m {
        Metric::Value(_) => Metric::Value(()),
        Metric::Unavailable(r) => Metric::Unavailable(r),
    }
}

impl MetricSource for FreqSource {
    fn name(&self) -> &'static str {
        "freq"
    }

    fn probe(&mut self, fs: &SysFs, dev: &Device) -> Capabilities {
        let mut fields = Vec::with_capacity(Field::ALL.len());
        for field in Field::ALL {
            fields.push((field.cap_name(), to_unit(read_field(fs, dev, field))));
        }
        Capabilities { fields }
    }

    fn sample(&mut self, fs: &SysFs, dev: &Device, _now: Instant, out: &mut Snapshot) {
        // Yalnızca `out.freq` dokunulur; diğer alanlar örnekleyicinindir.
        out.freq = FreqInfo {
            act: read_field(fs, dev, Field::Act),
            cur: read_field(fs, dev, Field::Cur),
            min: read_field(fs, dev, Field::Min),
            max: read_field(fs, dev, Field::Max),
            boost: read_field(fs, dev, Field::Boost),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(card: u32, driver: Driver) -> Device {
        Device {
            card,
            pci_slot: String::new(),
            device_id: 0x56a1,
            name: "test GPU".into(),
            driver,
        }
    }

    #[test]
    fn i915_yolu_dogrulanmis_bicimde() {
        let dev = device(1, Driver::I915);
        assert_eq!(
            path_for(&dev, Field::Act).as_deref(),
            Some("/sys/class/drm/card1/gt_act_freq_mhz")
        );
        assert_eq!(
            path_for(&dev, Field::Boost).as_deref(),
            Some("/sys/class/drm/card1/gt_boost_freq_mhz")
        );
    }

    #[test]
    fn xe_yolu_boost_haric_tam() {
        let dev = device(0, Driver::Xe);
        assert_eq!(
            path_for(&dev, Field::Cur).as_deref(),
            Some("/sys/class/drm/card0/device/tile0/gt0/freq0/cur_freq")
        );
        assert_eq!(path_for(&dev, Field::Boost), None);
    }

    #[test]
    fn u32_tasmasi_ayristirilamadi_der() {
        assert_eq!(fit_u32(u64::from(u32::MAX)), Metric::Value(u32::MAX));
        assert_eq!(
            fit_u32(u64::from(u32::MAX) + 1),
            Metric::Unavailable("ayrıştırılamadı")
        );
    }
}
