//! `--record`: TUI'deki her snapshot'ı CSV dosyasına bir satır olarak yazar.

use std::fs::File;
use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use arctop_core::{EngineClass, Metric, Snapshot};

/// Sabit sütun sırası; başlık satırı da bu sırayı izler.
const HEADER: [&str; 14] = [
    "elapsed_ms",
    "freq_act_mhz",
    "freq_cur_mhz",
    "power_w",
    "power_limit_w",
    "temp_c",
    "fan_rpm",
    "voltage_mv",
    "vram_used_bytes",
    "busy_render",
    "busy_copy",
    "busy_video",
    "busy_video_enhance",
    "busy_compute",
];

/// Snapshot'ları CSV olarak yazan kayıt aracı.
pub struct Recorder {
    writer: csv::Writer<File>,
    /// İlk snapshot'ın zamanı; `elapsed_ms` buna göre hesaplanır.
    origin: Option<Instant>,
}

impl Recorder {
    /// Dosyayı oluşturur (varsa üzerine yazar) ve başlık satırını yazar.
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        let file = File::create(path)
            .with_context(|| format!("kayıt dosyası açılamadı: {}", path.display()))?;
        let mut writer = csv::Writer::from_writer(file);
        writer
            .write_record(HEADER)
            .context("CSV başlığı yazılamadı")?;
        writer.flush().context("CSV başlığı yazılamadı")?;
        Ok(Self {
            writer,
            origin: None,
        })
    }

    /// Bir snapshot'ı satır olarak yazar ve hemen flush eder.
    /// Unavailable ya da olmayan metrikler boş hücre olarak yazılır.
    pub fn write(&mut self, s: &Snapshot) -> anyhow::Result<()> {
        let origin = *self.origin.get_or_insert(s.at);
        let elapsed = s.at.saturating_duration_since(origin).as_millis();
        let busy = |class: EngineClass| {
            s.engines
                .iter()
                .find(|e| e.class == class)
                .map(|e| e.busy_pct.to_string())
                .unwrap_or_default()
        };
        let row = [
            elapsed.to_string(),
            cell(&s.freq.act),
            cell(&s.freq.cur),
            cell(&s.power_w),
            cell(&s.power_limit_w),
            cell(&s.temp_c),
            cell(&s.fan_rpm),
            cell(&s.voltage_mv),
            cell(&s.vram_used),
            busy(EngineClass::Render),
            busy(EngineClass::Copy),
            busy(EngineClass::Video),
            busy(EngineClass::VideoEnhance),
            busy(EngineClass::Compute),
        ];
        self.writer
            .write_record(&row)
            .context("CSV satırı yazılamadı")?;
        self.writer.flush().context("CSV satırı yazılamadı")?;
        Ok(())
    }
}

/// Metriği hücre metnine çevirir; değer yoksa boş hücre.
fn cell<T: ToString>(m: &Metric<T>) -> String {
    m.value().map(|v| v.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// Testten sonra silinen geçici dosya.
    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            Self(std::env::temp_dir().join(format!(
                "arctop-record-{tag}-{}-{nanos}.csv",
                std::process::id()
            )))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn read_rows(path: &Path) -> Vec<Vec<String>> {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)
            .unwrap()
            .records()
            .map(|r| r.unwrap().iter().map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn writes_header_and_two_rows_with_empty_cells() {
        let tmp = TempFile::new("rows");
        let t0 = Instant::now();

        let mut first = Snapshot::empty(t0);
        first.power_w = Metric::Value(35.2);
        first.engines = vec![arctop_core::EngineUsage {
            class: EngineClass::Render,
            busy_pct: 12.5,
        }];
        let mut second = Snapshot::empty(t0 + Duration::from_millis(1000));
        second.fan_rpm = Metric::Value(720);

        let mut rec = Recorder::create(&tmp.0).unwrap();
        rec.write(&first).unwrap();
        rec.write(&second).unwrap();
        drop(rec);

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 3, "başlık + 2 satır bekleniyor: {rows:?}");
        assert_eq!(rows[0], HEADER);

        let r1 = &rows[1];
        assert_eq!(r1.len(), HEADER.len());
        assert_eq!(r1[0], "0");
        assert_eq!(r1[3], "35.2");
        assert_eq!(r1[9], "12.5");
        assert_eq!(r1[10], "", "olmayan motor boş hücre olmalı");
        assert_eq!(r1[6], "", "unavailable fan boş hücre olmalı");

        let r2 = &rows[2];
        assert_eq!(r2[0], "1000");
        assert_eq!(r2[6], "720");
        assert_eq!(r2[3], "", "unavailable güç boş hücre olmalı");
        assert_eq!(r2[9], "", "motor yoksa boş hücre olmalı");
    }

    #[test]
    fn create_overwrites_existing_file() {
        let tmp = TempFile::new("overwrite");
        std::fs::write(&tmp.0, "eski içerik\n").unwrap();
        let rec = Recorder::create(&tmp.0).unwrap();
        drop(rec);
        let content = std::fs::read_to_string(&tmp.0).unwrap();
        assert!(!content.contains("eski"));
        assert_eq!(content.lines().count(), 1, "yalnızca başlık olmalı");
    }
}
