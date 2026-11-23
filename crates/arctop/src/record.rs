//! `--record`: TUI'deki her snapshot'ı CSV dosyasına bir satır olarak yazar.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use arctop_core::{EngineClass, Metric, Snapshot};

/// Sabit sütun sırası; başlık satırı da bu sırayı izler.
///
/// `session_start` (RFC 3339, UTC) her kayıt oturumunda sabittir; `elapsed_ms`
/// her oturumda 0'dan başladığı için aynı dosyaya eklenen oturumlar bu
/// sütunla ayırt edilir.
const HEADER: [&str; 15] = [
    "session_start",
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
    /// Oturumun başlangıcı (RFC 3339, UTC); `session_start` sütununa yazılır.
    session_start: String,
    /// Dosya `session_start` sütunundan önceki eski biçimde: satırlar eski
    /// 14 sütunlu biçimde yazılır ki dosyada iki biçim karışmasın.
    legacy: bool,
    /// İlk snapshot'ın zamanı; `elapsed_ms` buna göre hesaplanır.
    origin: Option<Instant>,
}

impl Recorder {
    /// Dosyayı ekleme kipinde açar (yoksa oluşturur). Başlık satırı yalnızca
    /// dosya yeni ya da boşsa yazılır; mevcut içerik korunur.
    ///
    /// Mevcut dosyanın başlığı denetlenir: güncel başlıksa olduğu gibi eklenir;
    /// `session_start` öncesi eski başlıksa eski 14 sütunlu biçimde yazmaya
    /// devam edilir (bkz. [`Recorder::is_legacy`]); tanınmayan bir başlık varsa
    /// biçimler karışmasın diye hata verilir.
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        Self::open(path, SystemTime::now())
    }

    fn open(path: &Path, session_start: SystemTime) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("kayıt dosyası açılamadı: {}", path.display()))?;
        let len = file
            .metadata()
            .with_context(|| format!("kayıt dosyası okunamadı: {}", path.display()))?
            .len();
        let is_empty = len == 0;
        let mut legacy = false;
        if !is_empty {
            legacy = match existing_format(&mut file) {
                Some(Format::Current) => false,
                Some(Format::Legacy) => true,
                None => anyhow::bail!(
                    "{} mevcut bir arctop kaydı değil (başlık satırı tanınmadı); \
                     biçimler karışmasın diye ekleme yapılmadı. Yeni bir dosya adı kullanın.",
                    path.display()
                ),
            };
            // Son satır '\n' ile bitmiyorsa yeni satır eski satıra yapışmasın.
            let mut last = [0u8; 1];
            file.seek(SeekFrom::Start(len - 1))
                .and_then(|_| file.read_exact(&mut last))
                .with_context(|| format!("kayıt dosyası okunamadı: {}", path.display()))?;
            if last[0] != b'\n' {
                file.write_all(b"\n")
                    .with_context(|| format!("kayıt dosyasına yazılamadı: {}", path.display()))?;
            }
        }
        let mut writer = csv::Writer::from_writer(file);
        if is_empty {
            writer
                .write_record(HEADER)
                .context("CSV başlığı yazılamadı")?;
            writer.flush().context("CSV başlığı yazılamadı")?;
        }
        Ok(Self {
            writer,
            session_start: rfc3339_utc(session_start),
            legacy,
            origin: None,
        })
    }

    /// Dosya `session_start` sütunu olmayan eski biçimde mi? Öyleyse satırlar
    /// da eski biçimde yazılır ve çağıran kullanıcıyı uyarmalıdır.
    pub fn is_legacy(&self) -> bool {
        self.legacy
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
            self.session_start.clone(),
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
        // Eski biçimli dosyada ilk sütun (session_start) yazılmaz.
        let row = if self.legacy { &row[1..] } else { &row[..] };
        self.writer
            .write_record(row)
            .context("CSV satırı yazılamadı")?;
        self.writer.flush().context("CSV satırı yazılamadı")?;
        Ok(())
    }
}

/// Mevcut dosyanın başlık biçimi.
enum Format {
    /// Güncel başlık (`session_start` dahil).
    Current,
    /// `session_start` öncesi eski 14 sütunlu başlık.
    Legacy,
}

/// Dosyanın ilk satırını okuyup biçimini tanır; tanınmazsa `None`.
fn existing_format(file: &mut File) -> Option<Format> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(&*file);
    let first = reader.records().next()?.ok()?;
    let cells: Vec<&str> = first.iter().collect();
    if cells == HEADER {
        Some(Format::Current)
    } else if cells == HEADER[1..] {
        Some(Format::Legacy)
    } else {
        None
    }
}

/// `SystemTime` değerini RFC 3339 UTC metnine (`2026-11-23T12:34:56.789Z`) çevirir.
fn rfc3339_utc(t: SystemTime) -> String {
    let ms = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let (secs, millis) = (ms / 1000, ms % 1000);
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // Gün sayısından takvim tarihi (Howard Hinnant, civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
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
        assert!(
            r1[0].ends_with('Z'),
            "session_start RFC 3339 UTC: {}",
            r1[0]
        );
        assert_eq!(r1[1], "0");
        assert_eq!(r1[4], "35.2");
        assert_eq!(r1[10], "12.5");
        assert_eq!(r1[11], "", "olmayan motor boş hücre olmalı");
        assert_eq!(r1[7], "", "unavailable fan boş hücre olmalı");

        let r2 = &rows[2];
        assert_eq!(r2[0], r1[0], "aynı oturumda session_start sabit");
        assert_eq!(r2[1], "1000");
        assert_eq!(r2[7], "720");
        assert_eq!(r2[4], "", "unavailable güç boş hücre olmalı");
        assert_eq!(r2[10], "", "motor yoksa boş hücre olmalı");
    }

    #[test]
    fn create_appends_without_repeating_header() {
        let tmp = TempFile::new("append");
        let t0 = Instant::now();

        let mut rec = Recorder::create(&tmp.0).unwrap();
        rec.write(&Snapshot::empty(t0)).unwrap();
        drop(rec);

        let mut rec = Recorder::create(&tmp.0).unwrap();
        rec.write(&Snapshot::empty(t0)).unwrap();
        drop(rec);

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 3, "başlık bir kez + 2 satır: {rows:?}");
        assert_eq!(rows[0], HEADER);
        assert_ne!(rows[1], HEADER);
        assert_ne!(rows[2], HEADER);
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn rfc3339_formats_known_instants() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + Duration::from_millis(1_709_210_096_789)),
            "2024-02-29T12:34:56.789Z"
        );
        assert_eq!(rfc3339_utc(at(951_782_400)), "2000-02-29T00:00:00.000Z");
        assert_eq!(rfc3339_utc(at(4_102_444_799)), "2099-12-31T23:59:59.000Z");
    }

    #[test]
    fn appended_sessions_are_distinguishable_by_session_start() {
        let tmp = TempFile::new("sessions");
        let t0 = Instant::now();

        for start in [at(1_700_000_000), at(1_700_003_600)] {
            let mut rec = Recorder::open(&tmp.0, start).unwrap();
            assert!(!rec.is_legacy());
            rec.write(&Snapshot::empty(t0)).unwrap();
            rec.write(&Snapshot::empty(t0 + Duration::from_millis(500)))
                .unwrap();
        }

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 5, "{rows:?}");
        // Her oturumda elapsed_ms 0'dan başlar; session_start oturumları ayırır.
        assert_eq!(rows[1][..2], ["2023-11-14T22:13:20.000Z", "0"]);
        assert_eq!(rows[2][..2], ["2023-11-14T22:13:20.000Z", "500"]);
        assert_eq!(rows[3][..2], ["2023-11-14T23:13:20.000Z", "0"]);
        assert_eq!(rows[4][..2], ["2023-11-14T23:13:20.000Z", "500"]);
    }

    #[test]
    fn legacy_file_keeps_old_format_and_reports_it() {
        let tmp = TempFile::new("legacy");
        let old_header = HEADER[1..].join(",");
        std::fs::write(
            &tmp.0,
            format!("{old_header}\n0,1,2,3,4,5,6,7,8,9,10,11,12,13\n"),
        )
        .unwrap();

        let mut rec = Recorder::create(&tmp.0).unwrap();
        assert!(rec.is_legacy());
        rec.write(&Snapshot::empty(Instant::now())).unwrap();
        drop(rec);

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert_eq!(rows[0], HEADER[1..]);
        assert!(
            rows.iter().all(|r| r.len() == 14),
            "biçim karışmamalı: {rows:?}"
        );
        assert_eq!(rows[2][0], "0", "ilk sütun elapsed_ms kalır");
    }

    #[test]
    fn unrecognized_existing_content_is_refused_untouched() {
        let tmp = TempFile::new("foreign");
        std::fs::write(&tmp.0, "eski içerik\n").unwrap();
        let err = Recorder::create(&tmp.0).err().expect("hata bekleniyor");
        assert!(err.to_string().contains("Yeni bir dosya adı"), "{err}");
        assert_eq!(std::fs::read_to_string(&tmp.0).unwrap(), "eski içerik\n");
    }

    #[test]
    fn create_writes_header_for_empty_file() {
        let empty = TempFile::new("empty");
        std::fs::write(&empty.0, "").unwrap();
        drop(Recorder::create(&empty.0).unwrap());
        let content = std::fs::read_to_string(&empty.0).unwrap();
        assert_eq!(content.lines().count(), 1, "boş dosyaya başlık yazılır");
        assert_eq!(content.trim_end(), HEADER.join(","));
    }

    #[test]
    fn create_terminates_unterminated_last_row() {
        let tmp = TempFile::new("noeol");
        std::fs::write(&tmp.0, HEADER.join(",")).unwrap();
        let mut rec = Recorder::create(&tmp.0).unwrap();
        rec.write(&Snapshot::empty(Instant::now())).unwrap();
        drop(rec);
        let content = std::fs::read_to_string(&tmp.0).unwrap();
        let mut lines = content.lines();
        assert_eq!(lines.next(), Some(HEADER.join(",").as_str()));
        assert!(lines.next().unwrap().contains(",0,"));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn legacy_file_with_unterminated_last_row_is_terminated_before_append() {
        let tmp = TempFile::new("legacy-noeol");
        let old_header = HEADER[1..].join(",");
        std::fs::write(
            &tmp.0,
            format!("{old_header}\n0,1,2,3,4,5,6,7,8,9,10,11,12,13"),
        )
        .unwrap();

        let mut rec = Recorder::create(&tmp.0).unwrap();
        assert!(rec.is_legacy());
        rec.write(&Snapshot::empty(Instant::now())).unwrap();
        drop(rec);

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert!(rows.iter().all(|r| r.len() == 14), "{rows:?}");
        assert_eq!(rows[1][0], "0");
    }

    #[test]
    fn current_file_with_unterminated_data_row_is_terminated_before_append() {
        let tmp = TempFile::new("current-noeol");
        let row = vec!["x"; HEADER.len()].join(",");
        std::fs::write(&tmp.0, format!("{}\n{row}", HEADER.join(","))).unwrap();

        let mut rec = Recorder::create(&tmp.0).unwrap();
        rec.write(&Snapshot::empty(Instant::now())).unwrap();
        drop(rec);

        let rows = read_rows(&tmp.0);
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert!(rows.iter().all(|r| r.len() == HEADER.len()), "{rows:?}");
    }
}
