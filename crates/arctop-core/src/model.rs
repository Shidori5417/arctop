//! Veri modeli: snapshot ve metrik tipleri (§4).

use std::fmt;
use std::str::FromStr;
use std::time::Instant;

use serde::Serialize;

use crate::error::ReadError;

/// Okunabilen ya da nedeniyle birlikte okunamayan bir metrik.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Metric<T> {
    /// Okunan değer.
    Value(T),
    /// Okunamadı; kısa neden.
    Unavailable(&'static str),
}

impl<T> Metric<T> {
    /// Değer varsa `Some(&T)` döndürür.
    pub fn value(&self) -> Option<&T> {
        match self {
            Metric::Value(v) => Some(v),
            Metric::Unavailable(_) => None,
        }
    }

    /// Değer okunabildi mi?
    pub fn is_available(&self) -> bool {
        matches!(self, Metric::Value(_))
    }
}

impl<T> From<Result<T, ReadError>> for Metric<T> {
    fn from(r: Result<T, ReadError>) -> Self {
        match r {
            Ok(v) => Metric::Value(v),
            Err(e) => Metric::Unavailable(e.reason()),
        }
    }
}

/// Çekirdek sürücüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Driver {
    /// `i915` sürücüsü.
    I915,
    /// `xe` sürücüsü.
    Xe,
}

impl fmt::Display for Driver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Driver::I915 => "i915",
            Driver::Xe => "xe",
        })
    }
}

/// Bilinmeyen sürücü adı hatası.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("bilinmeyen sürücü: {0}")]
pub struct ParseDriverError(pub String);

impl FromStr for Driver {
    type Err = ParseDriverError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "i915" => Ok(Driver::I915),
            "xe" => Ok(Driver::Xe),
            other => Err(ParseDriverError(other.to_string())),
        }
    }
}

/// Keşfedilen bir GPU.
#[derive(Debug, Clone, Serialize)]
pub struct Device {
    /// `cardN` numarası.
    pub card: u32,
    /// PCI yuvası (örn. `0000:03:00.0`).
    pub pci_slot: String,
    /// PCI device id.
    pub device_id: u16,
    /// Okunabilir ad.
    pub name: String,
    /// Sürücü.
    pub driver: Driver,
}

/// Motor sınıfı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EngineClass {
    /// Render (3B).
    Render,
    /// Kopyalama (blitter).
    Copy,
    /// Video kod çözme/çözme.
    Video,
    /// Video iyileştirme.
    VideoEnhance,
    /// Hesaplama.
    Compute,
}

impl EngineClass {
    /// fdinfo'daki sınıf adı (`drm-engine-<ad>`).
    pub fn as_fdinfo_key(&self) -> &'static str {
        match self {
            EngineClass::Render => "render",
            EngineClass::Copy => "copy",
            EngineClass::Video => "video",
            EngineClass::VideoEnhance => "video-enhance",
            EngineClass::Compute => "compute",
        }
    }
}

/// Bir motor sınıfının doluluğu.
#[derive(Debug, Clone, Serialize)]
pub struct EngineUsage {
    /// Motor sınıfı.
    pub class: EngineClass,
    /// Doluluk yüzdesi.
    pub busy_pct: f32,
}

/// Frekans bilgisi (MHz).
#[derive(Debug, Clone, Serialize)]
pub struct FreqInfo {
    /// Gerçekleşen frekans.
    pub act: Metric<u32>,
    /// İstenen frekans.
    pub cur: Metric<u32>,
    /// Alt sınır.
    pub min: Metric<u32>,
    /// Üst sınır.
    pub max: Metric<u32>,
    /// Boost frekansı.
    pub boost: Metric<u32>,
}

/// Süreç başına GPU kullanımı.
#[derive(Debug, Clone, Serialize)]
pub struct ProcUsage {
    /// Süreç kimliği.
    pub pid: u32,
    /// Süreç adı.
    pub comm: String,
    /// Motor doluluklar.
    pub engines: Vec<EngineUsage>,
    /// Kullanılan VRAM (bayt).
    pub vram_bytes: Metric<u64>,
}

/// Bir örnekleme anındaki tüm metrikler.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    /// Örnekleme zamanı (serileştirilmez).
    #[serde(skip)]
    pub at: Instant,
    /// Frekanslar.
    pub freq: FreqInfo,
    /// Güç (W).
    pub power_w: Metric<f32>,
    /// Güç sınırı (W).
    pub power_limit_w: Metric<f32>,
    /// Sıcaklık (°C).
    pub temp_c: Metric<f32>,
    /// Fan hızı (RPM).
    pub fan_rpm: Metric<u32>,
    /// Gerilim (mV).
    pub voltage_mv: Metric<u32>,
    /// Motor doluluğu.
    pub engines: Vec<EngineUsage>,
    /// Kullanılan VRAM (bayt).
    pub vram_used: Metric<u64>,
    /// Toplam VRAM (bayt).
    pub vram_total: Metric<u64>,
    /// Süreç kullanımları.
    pub procs: Vec<ProcUsage>,
}

impl Snapshot {
    /// Tüm metrikleri `Unavailable("örneklenmedi")` ile kurar.
    pub fn empty(at: Instant) -> Self {
        const NS: &str = "örneklenmedi";
        Self {
            at,
            freq: FreqInfo {
                act: Metric::Unavailable(NS),
                cur: Metric::Unavailable(NS),
                min: Metric::Unavailable(NS),
                max: Metric::Unavailable(NS),
                boost: Metric::Unavailable(NS),
            },
            power_w: Metric::Unavailable(NS),
            power_limit_w: Metric::Unavailable(NS),
            temp_c: Metric::Unavailable(NS),
            fan_rpm: Metric::Unavailable(NS),
            voltage_mv: Metric::Unavailable(NS),
            engines: Vec::new(),
            vram_used: Metric::Unavailable(NS),
            vram_total: Metric::Unavailable(NS),
            procs: Vec::new(),
        }
    }
}

/// Bir kaynağın `probe` sonucu: alan adı → okunabilir mi / neden değil.
#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    /// Alan adı ve durumu.
    pub fields: Vec<(&'static str, Metric<()>)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn metric_helpers() {
        let v: Metric<u32> = Metric::Value(3);
        assert_eq!(v.value(), Some(&3));
        assert!(v.is_available());
        let u: Metric<u32> = Metric::Unavailable("yok");
        assert_eq!(u.value(), None);
        assert!(!u.is_available());
    }

    #[test]
    fn metric_from_result() {
        let ok: Metric<u64> = Ok(5u64).into();
        assert_eq!(ok, Metric::Value(5));
        let err: Metric<u64> = Err(ReadError::NotFound(PathBuf::from("/x"))).into();
        assert_eq!(err, Metric::Unavailable("yok"));
    }

    #[test]
    fn driver_display_fromstr() {
        assert_eq!(Driver::I915.to_string(), "i915");
        assert_eq!(Driver::Xe.to_string(), "xe");
        assert_eq!("i915".parse::<Driver>(), Ok(Driver::I915));
        assert_eq!("xe".parse::<Driver>(), Ok(Driver::Xe));
        assert!("nouveau".parse::<Driver>().is_err());
    }

    #[test]
    fn engine_keys() {
        assert_eq!(EngineClass::Render.as_fdinfo_key(), "render");
        assert_eq!(EngineClass::Copy.as_fdinfo_key(), "copy");
        assert_eq!(EngineClass::Video.as_fdinfo_key(), "video");
        assert_eq!(EngineClass::VideoEnhance.as_fdinfo_key(), "video-enhance");
        assert_eq!(EngineClass::Compute.as_fdinfo_key(), "compute");
    }

    #[test]
    fn snapshot_empty_is_unavailable() {
        let s = Snapshot::empty(Instant::now());
        assert_eq!(s.power_w, Metric::Unavailable("örneklenmedi"));
        assert!(!s.freq.act.is_available());
        assert!(s.engines.is_empty() && s.procs.is_empty());
    }
}
