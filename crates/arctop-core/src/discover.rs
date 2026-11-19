//! Keşif: `/sys/class/drm` altındaki Intel GPU'ların bulunması (§2).

use crate::{Device, Driver, ReadError, SysFs, pci_ids};

/// Intel (vendor 0x8086) DRM kartlarını kart numarasına göre sıralı döndürür.
///
/// Yalnızca `cardN` biçimindeki girdiler incelenir; bağlayıcı girdileri
/// (`card1-DP-1` gibi), `renderD*` ve diğer girdiler atlanır. Bir kartın
/// `vendor`, `device` veya `driver` bilgisi okunamıyorsa ya da beklenen
/// biçimde değilse yalnızca o kart atlanır; tek bir kartın hatası
/// diğerlerini engellemez. `/sys/class/drm` dizininin kendisi okunamazsa
/// hata döner.
///
/// `pci_slot` alanı `/sys/class/drm/cardN/device` bağının hedef adının son
/// bileşenidir (örn. `0000:03:00.0`); bağ okunamazsa boş string'tir.
/// Fixture'larda `device` gerçek bir dizin olduğundan bu alan boş kalır.
pub fn discover(fs: &SysFs) -> Result<Vec<Device>, ReadError> {
    let mut devices = Vec::new();
    for entry in fs.read_dir("/sys/class/drm")? {
        // read_dir kök içermeyen mutlak yollar döndürür; ad son bileşendir.
        let Some(card_name) = entry.file_name().and_then(|n| n.to_str()) else {
            continue; // UTF-8 olmayan girdi adı
        };
        let Some(card) = card_number(card_name) else {
            continue; // card1-DP-1, renderD128, controlD128, version, ...
        };
        if let Some(device) = probe_card(fs, card_name, card) {
            devices.push(device);
        }
    }
    devices.sort_by_key(|d| d.card);
    Ok(devices)
}

/// Bir `cardN` girdisinden `Device` kurar; kart atlanacaksa `None`.
///
/// `None` döndüren durumlar: `vendor` okunamıyor ya da `0x8086` değil,
/// `device` kimliği okunamıyor ya da hex olarak ayrıştırılamıyor, sürücü
/// adı `i915`/`xe` değil.
fn probe_card(fs: &SysFs, card_name: &str, card: u32) -> Option<Device> {
    let base = format!("/sys/class/drm/{card_name}/device");

    if !matches!(
        fs.read_string(format!("{base}/vendor")).as_deref(),
        Ok("0x8086")
    ) {
        return None;
    }

    // `device` dosyası `0x56a1` biçimindedir; ondalık okuyan `read_u64`
    // burada yanlış değer verirdi, bu yüzden `read_string` + hex ayrıştırma.
    let device_id = parse_hex_u16(&fs.read_string(format!("{base}/device")).ok()?)?;

    // Bilinmeyen sürücü (örn. `nouveau`) kartın atlanmasına yol açar.
    let driver = fs
        .read_link_name(format!("{base}/driver"))
        .ok()?
        .parse::<Driver>()
        .ok()?;

    // Gerçek sistemde PCI yuvasına giden bağ; fixture'da dizindir, okunamaz.
    let pci_slot = fs.read_link_name(&base).unwrap_or_default();

    let name = match pci_ids::name_for(device_id) {
        Some(known) => known.to_owned(),
        None => format!("Intel GPU {device_id:#06x}"),
    };

    Some(Device {
        card,
        pci_slot,
        device_id,
        name,
        driver,
    })
}

/// `cardN` girdi adını kart numarasına çevirir; başka biçimde `None`.
///
/// `card0`/`card1` → `Some(0)`/`Some(1)`; `card1-DP-1`, `renderD128`,
/// `controlD128`, `version` → `None`.
fn card_number(name: &str) -> Option<u32> {
    let digits = name.strip_prefix("card")?;
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        digits.parse().ok() // `u32` taşması da kartın atlanması demektir
    } else {
        None
    }
}

/// sysfs'teki `0x`-önekli hex değeri (`0x56a1`) `u16`'ya çevirir.
fn parse_hex_u16(value: &str) -> Option<u16> {
    let digits = value.strip_prefix("0x").unwrap_or(value);
    u16::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kart_adi_filtresi() {
        assert_eq!(card_number("card0"), Some(0));
        assert_eq!(card_number("card1"), Some(1));
        assert_eq!(card_number("card1-DP-1"), None);
        assert_eq!(card_number("renderD128"), None);
        assert_eq!(card_number("controlD64"), None);
        assert_eq!(card_number("version"), None);
        assert_eq!(card_number("card"), None);
        assert_eq!(card_number("cardX"), None);
        assert_eq!(card_number("card99999999999"), None); // u32 taşması
    }

    #[test]
    fn hex_ayristirma() {
        assert_eq!(parse_hex_u16("0x56a1"), Some(0x56a1));
        assert_eq!(parse_hex_u16("56a1"), Some(0x56a1));
        assert_eq!(parse_hex_u16("0x12345"), None); // u16 sınırı aşılır
        assert_eq!(parse_hex_u16("0xzz"), None);
    }

    #[test]
    fn drm_dizini_yoksa_hata() {
        let fs = SysFs::with_root("/nonexistent-arctop-discover");
        assert!(matches!(discover(&fs), Err(ReadError::NotFound(_))));
    }
}
