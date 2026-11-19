//! SysFs soyutlaması: sysfs/proc okumaları ve test için kök yolu enjeksiyonu (§1).
//!
//! Projedeki tek `std::fs` kullanım noktası burasıdır (PMU hariç).

use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::error::ReadError;

/// Kök dizini enjekte edilebilen dosya sistemi okuyucusu.
///
/// Gerçek sistemde kök `/`, testlerde fixture dizinidir. Yollar mutlak verilir
/// (`/sys/class/drm/card1/...`) ve kök ile birleştirilir.
#[derive(Debug, Clone)]
pub struct SysFs {
    root: PathBuf,
}

impl SysFs {
    /// Gerçek dosya sistemi (kök = `/`).
    pub fn real() -> Self {
        Self {
            root: PathBuf::from("/"),
        }
    }

    /// Verilen dizini kök kabul eden örnek (fixture'lar için).
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Mutlak yolu kök ile birleştirir (baştaki `/` atılır).
    fn full(&self, p: &Path) -> PathBuf {
        let rel: PathBuf = p
            .components()
            .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(_)))
            .collect();
        self.root.join(rel)
    }

    /// `std::io::Error`'u `ReadError`'a çevirir; `p` kök içermeyen yoldur.
    fn map_io(p: &Path, e: std::io::Error) -> ReadError {
        match e.kind() {
            ErrorKind::NotFound => ReadError::NotFound(p.to_path_buf()),
            ErrorKind::PermissionDenied => ReadError::PermissionDenied(p.to_path_buf()),
            _ => ReadError::Io {
                path: p.to_path_buf(),
                source: e,
            },
        }
    }

    /// Dosyayı `u64` olarak okur. İçerik kırpılır; sonrasında birim vb. kalırsa `Parse` hatası.
    pub fn read_u64(&self, p: impl AsRef<Path>) -> Result<u64, ReadError> {
        let p = p.as_ref();
        let s = self.read_string(p)?;
        s.trim().parse::<u64>().map_err(|_| ReadError::Parse {
            path: p.to_path_buf(),
            value: s,
        })
    }

    /// Dosyayı metin olarak okur; sondaki `\n` kırpılır.
    pub fn read_string(&self, p: impl AsRef<Path>) -> Result<String, ReadError> {
        let p = p.as_ref();
        let mut s = std::fs::read_to_string(self.full(p)).map_err(|e| Self::map_io(p, e))?;
        while s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        Ok(s)
    }

    /// Dizin girdilerini kök içermeyen mutlak yollar olarak, sıralı döndürür.
    pub fn read_dir(&self, p: impl AsRef<Path>) -> Result<Vec<PathBuf>, ReadError> {
        let p = p.as_ref();
        let rd = std::fs::read_dir(self.full(p)).map_err(|e| Self::map_io(p, e))?;
        let mut out = Vec::new();
        for entry in rd {
            let entry = entry.map_err(|e| Self::map_io(p, e))?;
            out.push(p.join(entry.file_name()));
        }
        out.sort();
        Ok(out)
    }

    /// Sembolik bağın hedefinin son bileşenini döndürür.
    ///
    /// Gerçek bir bağ değilse (fixture) `<p>.name` dosyasının içeriğine düşer.
    pub fn read_link_name(&self, p: impl AsRef<Path>) -> Result<String, ReadError> {
        let p = p.as_ref();
        match std::fs::read_link(self.full(p)) {
            Ok(target) => target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| ReadError::Parse {
                    path: p.to_path_buf(),
                    value: target.to_string_lossy().into_owned(),
                }),
            // NotFound: bağ yok; InvalidInput: sembolik bağ değil. İkisinde de .name'e düş.
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::InvalidInput) => {
                let mut name = p.as_os_str().to_os_string();
                name.push(".name");
                match self.read_string(PathBuf::from(name)) {
                    Ok(s) => Ok(s.trim().to_string()),
                    Err(ReadError::NotFound(_)) => Err(ReadError::NotFound(p.to_path_buf())),
                    Err(other) => Err(other),
                }
            }
            Err(e) => Err(Self::map_io(p, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_strips_leading_slash() {
        let fs = SysFs::with_root("/r");
        assert_eq!(fs.full(Path::new("/a/b")), PathBuf::from("/r/a/b"));
        assert_eq!(fs.full(Path::new("a/b")), PathBuf::from("/r/a/b"));
    }

    #[test]
    fn missing_is_not_found() {
        let fs = SysFs::with_root("/nonexistent-arctop-root");
        assert!(matches!(fs.read_u64("/x"), Err(ReadError::NotFound(_))));
    }
}
