//! PCI device id → okunabilir ad tablosu: Intel Arc/Xe dGPU'ları.
//!
//! Adlar, pci.ids veritabanının (https://pci-ids.ucw.cz/, 2026-11-19 sürümü)
//! `8086:56xx` (DG2/Alchemist) ve `8086:e2xx` (Battlemage) girdilerinden
//! alınmıştır; doğrulanmamış kimlik koda girmez. Pazarlama adı olmayan OEM
//! kimlikleri (`DG2 [Arc Xe Graphics]` gibi) tabloda değildir; onlar
//! `discover` tarafında `Intel GPU 0x....` genel adına düşer.

/// Bilinen Intel Arc/Xe aygıtları için okunabilir ad; bilinmiyorsa `None`.
pub fn name_for(device_id: u16) -> Option<&'static str> {
    static TABLE: &[(u16, &str)] = &[
        // DG2 (Alchemist), mobil
        (0x5690, "Intel Arc A770M"),
        (0x5691, "Intel Arc A730M"),
        (0x5692, "Intel Arc A550M"),
        (0x5693, "Intel Arc A370M"),
        (0x5694, "Intel Arc A350M"),
        (0x5695, "Intel Iris Xe MAX A200M"),
        (0x5696, "Intel Arc A570M"),
        (0x5697, "Intel Arc A530M"),
        // DG2 (Alchemist), masaüstü
        (0x56a0, "Intel Arc A770"),
        (0x56a1, "Intel Arc A750"),
        (0x56a2, "Intel Arc A580"),
        (0x56a5, "Intel Arc A380"),
        (0x56a6, "Intel Arc A310"),
        // DG2 (Alchemist), profesyonel
        (0x56b0, "Intel Arc Pro A30M"),
        (0x56b1, "Intel Arc Pro A40/A50"),
        (0x56b2, "Intel Arc Pro A60M"),
        (0x56b3, "Intel Arc Pro A60"),
        // DG2 (Alchemist), E-serisi
        (0x56ba, "Intel Arc A380E"),
        (0x56bb, "Intel Arc A310E"),
        (0x56bc, "Intel Arc A370E"),
        (0x56bd, "Intel Arc A350E"),
        (0x56be, "Intel Arc A750E"),
        (0x56bf, "Intel Arc A580E"),
        // ATS-M, veri merkezi
        (0x56c0, "Intel Data Center GPU Flex 170"),
        (0x56c1, "Intel Data Center GPU Flex 140"),
        (0x56c2, "Intel Data Center GPU Flex 170V"),
        // Battlemage
        (0xe20b, "Intel Arc B580"),
        (0xe20c, "Intel Arc B570"),
        (0xe211, "Intel Arc Pro B60"),
        (0xe212, "Intel Arc Pro B50"),
        (0xe222, "Intel Arc Pro B65"),
        (0xe223, "Intel Arc Pro B70"),
    ];
    TABLE
        .iter()
        .find(|entry| entry.0 == device_id)
        .map(|entry| entry.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilinen_id() {
        assert_eq!(name_for(0x56a1), Some("Intel Arc A750"));
        assert_eq!(name_for(0x56a0), Some("Intel Arc A770"));
        assert_eq!(name_for(0x56a5), Some("Intel Arc A380"));
        assert_eq!(name_for(0xe20b), Some("Intel Arc B580"));
        assert_eq!(name_for(0xe20c), Some("Intel Arc B570"));
    }

    #[test]
    fn bilinmeyen_id() {
        assert_eq!(name_for(0x1234), None);
        assert_eq!(name_for(0), None);
    }
}
