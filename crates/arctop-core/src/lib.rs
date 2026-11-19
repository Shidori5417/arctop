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

#[cfg(test)]
mod tests {
    #[test]
    fn it_builds() {}
}
