//! Görünümlerin ortak biçimlendiricileri.
// Görünüm kartları (T-010..T-013) gelene dek kullanılmayanlar olabilir.
#![allow(dead_code)]

use std::fmt::Display;

use arctop_core::Metric;

/// Okunamayan metriklerin gösterimi.
pub const DASH: &str = "—";

/// `Value` ise "değer birim", `Unavailable` ise "—". Birim boşsa yalnızca değer.
pub fn metric<T: Display>(m: &Metric<T>, unit: &str) -> String {
    match m {
        Metric::Value(v) if unit.is_empty() => v.to_string(),
        Metric::Value(v) => format!("{v} {unit}"),
        Metric::Unavailable(_) => DASH.to_string(),
    }
}

/// Bayt sayısını MiB (tam sayı) ya da ≥1 GiB için GiB (1 ondalık) olarak yazar.
pub fn bytes(b: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * MIB;
    if b >= GIB {
        format!("{:.1} GiB", b as f64 / GIB as f64)
    } else {
        format!("{} MiB", b / MIB)
    }
}

/// Yüzdeyi tek ondalıkla yazar.
pub fn pct(f: f32) -> String {
    format!("{f:.1}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_formats() {
        assert_eq!(metric(&Metric::Value(1200u32), "MHz"), "1200 MHz");
        assert_eq!(metric(&Metric::Value(7u32), ""), "7");
        assert_eq!(metric::<u32>(&Metric::Unavailable("yok"), "MHz"), "—");
    }

    #[test]
    fn bytes_and_pct() {
        assert_eq!(bytes(0), "0 MiB");
        assert_eq!(bytes(300 * 1024 * 1024), "300 MiB");
        assert_eq!(bytes(1536 * 1024 * 1024), "1.5 GiB");
        assert_eq!(pct(12.345), "12.3%");
    }
}
