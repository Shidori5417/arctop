//! Metrik kaynakları: her kaynak ayrı bir alt modülde (§3).

pub mod fdinfo;
pub mod freq;
pub mod hwmon;

use std::time::Instant;

use crate::model::{Capabilities, Device, Snapshot};
use crate::sysfs::SysFs;

/// Bir metrik kaynağı. Örnekleyici iş parçacığında sırayla çağrılır.
pub trait MetricSource: Send {
    /// Kaynağın kısa adı.
    fn name(&self) -> &'static str;
    /// Bu cihazda ne okunabildiğini bir kez belirler.
    fn probe(&mut self, fs: &SysFs, dev: &Device) -> Capabilities;
    /// Snapshot'taki kendi alanlarını doldurur. Δ hesabı için iç durum tutabilir.
    fn sample(&mut self, fs: &SysFs, dev: &Device, now: Instant, out: &mut Snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl MetricSource for Dummy {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn probe(&mut self, _: &SysFs, _: &Device) -> Capabilities {
            Capabilities { fields: vec![] }
        }
        fn sample(&mut self, _: &SysFs, _: &Device, _: Instant, _: &mut Snapshot) {}
    }

    #[test]
    fn trait_is_object_safe_and_send() {
        let b: Box<dyn MetricSource> = Box::new(Dummy);
        assert_eq!(b.name(), "dummy");
    }
}
