//! Kütüphane hata tipleri (`thiserror`).

use std::path::PathBuf;

/// `SysFs` okumalarında oluşabilen hatalar. Yollar kök (root) içermeyen mutlak yollardır.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// Dosya ya da dizin yok.
    #[error("bulunamadı: {0}")]
    NotFound(PathBuf),
    /// Okuma izni yok.
    #[error("izin yok: {0}")]
    PermissionDenied(PathBuf),
    /// İçerik beklenen biçimde ayrıştırılamadı.
    #[error("ayrıştırılamadı: {path} içeriği {value:?}")]
    Parse {
        /// Okunan dosyanın yolu.
        path: PathBuf,
        /// Ayrıştırılamayan (kırpılmış) içerik.
        value: String,
    },
    /// Diğer G/Ç hataları.
    #[error("G/Ç hatası: {path}: {source}")]
    Io {
        /// İlgili yol.
        path: PathBuf,
        /// Kaynak hata.
        source: std::io::Error,
    },
}

impl ReadError {
    /// `Metric::Unavailable` için kısa neden metni.
    pub fn reason(&self) -> &'static str {
        match self {
            ReadError::NotFound(_) => "yok",
            ReadError::PermissionDenied(_) => "izin yok",
            ReadError::Parse { .. } => "ayrıştırılamadı",
            ReadError::Io { .. } => "G/Ç hatası",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons() {
        let p = PathBuf::from("/x");
        assert_eq!(ReadError::NotFound(p.clone()).reason(), "yok");
        assert_eq!(ReadError::PermissionDenied(p.clone()).reason(), "izin yok");
        let parse = ReadError::Parse {
            path: p.clone(),
            value: "a".into(),
        };
        assert_eq!(parse.reason(), "ayrıştırılamadı");
        let io = ReadError::Io {
            path: p,
            source: std::io::Error::other("x"),
        };
        assert_eq!(io.reason(), "G/Ç hatası");
    }
}
