//! Yardım görünümü: tuş kısayolları ve eksik metriklerin nedenleri (§6).

use arctop_core::Metric;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::fmt;
use crate::ui::theme;

/// Kısayol listesi: (tuş, açıklama). Sıra SPEC "Kısayollar" ile aynıdır.
const KEYS: &[(&str, &str)] = &[
    ("1-3", "görünüm"),
    ("Tab", "sonraki görünüm"),
    ("s", "sıralama sütunu"),
    ("/", "filtre"),
    ("+/-", "örnekleme aralığı"),
    ("g", "60 sn / 5 dk"),
    ("c", "kart değiştir"),
    ("F1", "yardım"),
    ("q", "çıkış"),
];

/// `c` satırının altında gösterilen not (yalnızca tek GPU sürümü).
const C_NOTE: &str = "(birden fazla GPU varsa — henüz yok)";
/// Alt not: PMU izinleri.
const FOOTNOTE: &str = "Root gerekmez. PMU için perf_event_paranoid <= 0 veya CAP_PERFMON gerekir (henüz kullanılmıyor).";
/// Eksik metrik yokken gösterilen metin.
const ALL_OK: &str = "Tüm metrikler okunabiliyor";
/// Henüz snapshot yokken gösterilen metin.
const WAITING: &str = "örnek bekleniyor…";

/// Görünümü `area` içine çizer.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title_style(theme::title())
        .title(" Yardım ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [body, note] = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(inner);
    let [keys_area, missing_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Min(0)]).areas(body);

    f.render_widget(
        Paragraph::new(key_lines()).wrap(Wrap { trim: false }),
        keys_area,
    );
    f.render_widget(
        Paragraph::new(missing_lines(app)).wrap(Wrap { trim: false }),
        missing_area,
    );
    f.render_widget(
        Paragraph::new(Line::styled(FOOTNOTE, theme::dim())).wrap(Wrap { trim: true }),
        note,
    );
}

/// Sol sütun: başlık ve kısayol satırları.
fn key_lines() -> Vec<Line<'static>> {
    let mut v = vec![Line::styled("Kısayollar", theme::title())];
    for (key, desc) in KEYS {
        v.push(Line::from(vec![
            Span::styled(format!("{key:<6}"), theme::title()),
            Span::raw(*desc),
        ]));
        if *key == "c" {
            v.push(Line::styled(format!("      {C_NOTE}"), theme::dim()));
        }
    }
    v
}

/// Sağ sütun: başlık, eksik metrik satırları veya durum metni.
fn missing_lines(app: &App) -> Vec<Line<'static>> {
    let mut v = vec![Line::styled("Eksik metrikler", theme::title())];
    if app.last.is_none() {
        v.push(Line::styled(WAITING, theme::dim()));
    }
    let rows = missing(app);
    if rows.is_empty() {
        if app.last.is_some() {
            v.push(Line::styled(ALL_OK, theme::dim()));
        }
        return v;
    }
    for (name, reason) in rows {
        v.push(Line::from(vec![
            Span::raw(name),
            Span::raw(" "),
            Span::styled(fmt::DASH, theme::dim()),
            Span::styled(format!(" {reason}"), theme::dim()),
        ]));
    }
    v
}

/// Yetenek adı, alan adı ve görünen ad. Aynı metrik hem yetenek hem alan adıyla
/// geldiğinde ikisi de aynı görünen adla tekilleşir. Listede olmayan yetenekler
/// (ör. `freq.*`, `frekans.*`) adlarıyla görünür.
const METRIC_LABELS: &[(&str, &str, &str)] = &[
    ("hwmon.power", "power_w", "güç"),
    ("hwmon.power_limit", "power_limit_w", "limit"),
    ("hwmon.temp", "temp_c", "sıcaklık"),
    ("hwmon.fan", "fan_rpm", "fan"),
    ("hwmon.voltage", "voltage_mv", "voltaj"),
    ("fdinfo.vram_used", "vram_used", "VRAM kullanımı"),
    ("fdinfo.vram_total", "vram_total", "VRAM toplamı"),
];

/// Bir yetenek adı veya alan adı için görünen adı döndürür; tabloda yoksa girdinin kendisi.
fn label_of(key: &str) -> &str {
    METRIC_LABELS
        .iter()
        .find(|(cap, field, _)| *cap == key || *field == key)
        .map_or(key, |(_, _, label)| label)
}

/// Eksik metrikleri `(görünen ad, neden)` olarak toplar. Önce yetenekler, sonra son snapshot.
/// Aynı görünen ad yalnızca bir kez eklenir; ilk eklenen (yetenek) nedeni kalır.
fn missing(app: &App) -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for caps in &app.capabilities {
        for (name, m) in &caps.fields {
            check(&mut out, label_of(name), m);
        }
    }
    if let Some(s) = &app.last {
        check(&mut out, "frekans.act", &s.freq.act);
        check(&mut out, "frekans.cur", &s.freq.cur);
        check(&mut out, "frekans.min", &s.freq.min);
        check(&mut out, "frekans.max", &s.freq.max);
        check(&mut out, "frekans.boost", &s.freq.boost);
        check(&mut out, label_of("power_w"), &s.power_w);
        check(&mut out, label_of("power_limit_w"), &s.power_limit_w);
        check(&mut out, label_of("temp_c"), &s.temp_c);
        check(&mut out, label_of("fan_rpm"), &s.fan_rpm);
        check(&mut out, label_of("voltage_mv"), &s.voltage_mv);
        check(&mut out, label_of("vram_used"), &s.vram_used);
        check(&mut out, label_of("vram_total"), &s.vram_total);
    }
    out
}

/// Metrik `Unavailable` ise listeye ekler.
fn check<T>(out: &mut Vec<(String, &'static str)>, name: &str, m: &Metric<T>) {
    if let Metric::Unavailable(reason) = m {
        add(out, name, reason);
    }
}

/// Görünen ad listede yoksa `(ad, neden)` çiftini ekler.
fn add(out: &mut Vec<(String, &'static str)>, name: &str, reason: &'static str) {
    if !out.iter().any(|(n, _)| n.as_str() == name) {
        out.push((name.to_string(), reason));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arctop_core::{Capabilities, FreqInfo, Snapshot};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::time::Instant;

    fn draw(app: &App, w: u16, h: u16) -> Terminal<TestBackend> {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| render(f, f.area(), app)).unwrap();
        t
    }

    fn text(t: &Terminal<TestBackend>) -> String {
        t.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn snapshot_full_screen() {
        let t = draw(&App::sample(), 100, 30);
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn fits_80x24_without_panic() {
        let text = text(&draw(&App::sample(), 80, 24));
        assert!(text.contains("Kısayollar"));
        assert!(text.contains("çıkış"));
    }

    #[test]
    fn waiting_without_snapshot_does_not_panic() {
        let mut app = App::sample();
        app.last = None;
        let text = text(&draw(&app, 80, 24));
        assert!(text.contains(WAITING));
    }

    #[test]
    fn sample_lists_missing_metrics_once() {
        let rows = missing(&App::sample());
        // Yetenek "fan_rpm" ile snapshot alanı tek "fan" satırına düşer.
        let fan = rows.iter().filter(|(n, _)| n == "fan").count();
        let fan_rpm = rows.iter().filter(|(n, _)| n == "fan_rpm").count();
        assert_eq!(fan, 1);
        assert_eq!(fan_rpm, 0);
        assert!(
            rows.iter()
                .any(|(n, r)| n == "frekans.boost" && *r == "yok")
        );
    }

    #[test]
    fn add_dedupes_by_label() {
        let mut out = Vec::new();
        add(&mut out, "fan", "yok");
        add(&mut out, "fan", "yok");
        add(&mut out, "fan", "sysfs'de yok");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, "yok", "ilk eklenen neden korunmalı");
    }

    #[test]
    fn capability_and_field_rows_merge_under_one_label() {
        let mut app = App::sample();
        app.capabilities = vec![Capabilities {
            fields: vec![
                ("hwmon.power", Metric::Unavailable("hwmon yok")),
                ("fdinfo.vram_total", Metric::Unavailable("sysfs'de yok")),
            ],
        }];
        app.last = Some(Snapshot::empty(Instant::now()));
        let rows = missing(&app);
        let power = rows.iter().filter(|(n, _)| n == "güç").count();
        let vram_total = rows.iter().filter(|(n, _)| n == "VRAM toplamı").count();
        assert_eq!(power, 1, "güç tek satır olmalı: {rows:?}");
        assert_eq!(vram_total, 1, "VRAM toplamı tek satır olmalı: {rows:?}");
        assert!(
            !rows
                .iter()
                .any(|(n, _)| n == "power_w" || n == "vram_total" || n == "hwmon.power"),
            "ham ad görünmemeli: {rows:?}"
        );
        assert!(
            rows.iter()
                .any(|(n, r)| n == "VRAM toplamı" && *r == "sysfs'de yok")
        );
    }

    #[test]
    fn no_panic_at_small_sizes() {
        // 40x10 ve 20x4: örnekli, örneksiz ve eksik metrikli durumlar panik atmamalı.
        for (w, h) in [(40, 10), (20, 4)] {
            draw(&App::sample(), w, h);
            let mut waiting = App::sample();
            waiting.last = None;
            draw(&waiting, w, h);
        }
    }

    #[test]
    fn all_readable_shows_ok_text() {
        let mut app = App::sample();
        app.capabilities.clear();
        let mut s = Snapshot::empty(Instant::now());
        s.freq = FreqInfo {
            act: Metric::Value(1),
            cur: Metric::Value(1),
            min: Metric::Value(1),
            max: Metric::Value(1),
            boost: Metric::Value(1),
        };
        s.power_w = Metric::Value(1.0);
        s.power_limit_w = Metric::Value(1.0);
        s.temp_c = Metric::Value(1.0);
        s.fan_rpm = Metric::Value(1);
        s.voltage_mv = Metric::Value(1);
        s.vram_used = Metric::Value(1);
        s.vram_total = Metric::Value(1);
        app.last = Some(s);
        let text = text(&draw(&app, 100, 30));
        assert!(text.contains(ALL_OK));
    }
}
