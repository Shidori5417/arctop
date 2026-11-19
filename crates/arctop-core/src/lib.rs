//! arctop-core: keşif, metrik kaynakları, örnekleyici ve geçmiş. UI bilmez.

#![warn(missing_docs)]

pub mod discover;
pub mod error;
pub mod history;
pub mod model;
pub mod pci_ids;
pub mod sampler;
pub mod source;
pub mod sysfs;

pub use error::ReadError;
pub use model::{
    Capabilities, Device, Driver, EngineClass, EngineUsage, FreqInfo, Metric, ParseDriverError,
    ProcUsage, Snapshot,
};
pub use source::MetricSource;
pub use sysfs::SysFs;

#[cfg(test)]
mod tests {
    #[test]
    fn it_builds() {}
}
