//! Yardım görünümü: modal pencere, kısayollar ve eksik metriklerin nedenleri (docs/TUI-DESIGN.md §7).

use arctop_core::Metric;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};

use crate::app::App;
use crate::fmt;
use crate::ui::theme::{self, Tok};
use crate::ui::widgets;

/// Kısayol listesi: (tuş, açıklama). İki sütunlu ızgarada önce sol, sonra sağ sütun dolar.
const KEYS: &[(&str, &str)] = &[
    ("1-3", "görünüm"),
    ("Tab", "sonraki"),
    ("↑↓ j k", "seç"),
    ("PgUp/PgDn", "sayfa"),
    ("Home/End", "başa/sona"),
    ("s", "sırala"),
    ("/", "filtre"),
    ("+/-", "aralık"),
    ("g", "60 sn / 5 dk"),
    ("F1", "yardım"),
    ("Esc", "kapat"),
    ("q", "çıkış"),
];

/// Tuş çipinin sabit genişliği (hücre). Bu uzunluğu aşan tuşlar tek boşlukla izlenir.
const CHIP_W: usize = 9;
/// Alt not: PMU izinleri.
const FOOTNOTE: &str = "Root gerekmez. PMU için perf_event_paranoid <= 0 veya CAP_PERFMON gerekir (henüz kullanılmıyor).";
/// Eksik metrik yokken gösterilen metin.
const ALL_OK: &str = "Tüm metrikler okunabiliyor";
/// Henüz snapshot yokken gösterilen metin.
const WAITING: &str = "örnek bekleniyor…";
/// Bölüm başlıkları.
const SECTION_KEYS: &str = "Kısayollar";
const SECTION_MISSING: &str = "Eksik metrikler";

/// Modal pencereyi `area` (gövde) üzerine ortalayıp çizer; arkadaki görünüm
/// çağıran tarafından zaten çizilmiştir. Genişlik `min(76, alan-4)`, yükseklik içerik kadar.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let w = 76.min(area.width.saturating_sub(4));
    // Kenarlık (2) ve yatay iç boşluk (2) düşülmüş iç genişlik.
    let inner_w = usize::from(w.saturating_sub(4)).max(1);
    let col_w = (inner_w / 2).max(1);

    let (left, right) = key_columns();
    let grid_h = wrapped_height(&left, col_w).max(wrapped_height(&right, col_w));
    let rows = if app.last.is_some() {
        missing_rows(app)
    } else {
        Vec::new()
    };
    let body_h = if app.last.is_some() {
        wrapped_height(&rows, inner_w)
    } else {
        1
    };
    let note_h = wrapped_height(&[Line::raw(FOOTNOTE)], inner_w);
    // Başlık + ızgara + ayraç + başlık + gövde + not; kenarlık için +2.
    let content_h = 1 + grid_h + 1 + 1 + body_h + note_h;
    let h = to_u16(content_h + 2);
    let modal = widgets::centered(area, w, h);
    f.render_widget(Clear, modal);

    let block = widgets::panel_with(
        "Yardım",
        Line::styled(" F1/Esc kapat ", theme::fg(Tok::Muted)),
    )
    .border_style(theme::border_focus());
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let [head_k, grid, sep, head_m, body, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(to_u16(grid_h)),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(to_u16(body_h)),
        Constraint::Length(to_u16(note_h)),
    ])
    .areas(inner);

    f.render_widget(
        Paragraph::new(Line::styled(SECTION_KEYS, theme::bold(Tok::Accent))),
        head_k,
    );
    let [left_a, right_a] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Min(0)]).areas(grid);
    f.render_widget(Paragraph::new(left).wrap(Wrap { trim: false }), left_a);
    f.render_widget(Paragraph::new(right).wrap(Wrap { trim: false }), right_a);

    f.render_widget(
        Paragraph::new(Line::styled(
            "─".repeat(usize::from(sep.width)),
            theme::fg(Tok::Surface),
        )),
        sep,
    );

    f.render_widget(
        Paragraph::new(Line::styled(SECTION_MISSING, theme::bold(Tok::Accent))),
        head_m,
    );
    if app.last.is_none() {
        f.render_widget(widgets::empty_state(WAITING), body);
    } else {
        f.render_widget(Paragraph::new(rows).wrap(Wrap { trim: false }), body);
    }

    f.render_widget(
        Paragraph::new(Line::styled(FOOTNOTE, theme::fg(Tok::Muted))).wrap(Wrap { trim: true }),
        note,
    );
}

/// `usize` değeri `u16`'ya sığdırır; taşarsa en büyük değer.
fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Satırların `col_w` genişliğinde sarıldığında kaplayacağı yaklaşık satır sayısı.
fn wrapped_height(lines: &[Line], col_w: usize) -> usize {
    lines
        .iter()
        .map(|l| l.width().div_ceil(col_w.max(1)).max(1))
        .sum()
}

/// Kısayolları iki sütuna böler: önce sol, sonra sağ sütun.
fn key_columns() -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let (l, r) = KEYS.split_at(KEYS.len().div_ceil(2));
    (key_lines(l), key_lines(r))
}

/// Bir sütunun satırları: tuş çipi (Accent kalın) + açıklama.
fn key_lines(keys: &[(&'static str, &'static str)]) -> Vec<Line<'static>> {
    keys.iter()
        .map(|&(key, desc)| {
            Line::from(vec![
                Span::styled(chip(key), theme::bold(Tok::Accent)),
                Span::raw(desc),
            ])
        })
        .collect()
}

/// Tuş çipi: `CHIP_W` hücreye doldurulur, arkasına bir boşluk eklenir. `CHIP_W`'den uzun
/// tuşlar çipi taşırır; açıklamaları o satırda bir kayar.
fn chip(key: &str) -> String {
    let pad = CHIP_W.saturating_sub(key.chars().count()) + 1;
    format!("{key}{}", " ".repeat(pad))
}

/// Eksik metrik satırları: `● ad` (Warn) + ` — neden` (Muted). Hiç yoksa tek "okunabiliyor" satırı.
fn missing_rows(app: &App) -> Vec<Line<'static>> {
    let rows = missing(app);
    if rows.is_empty() {
        return vec![Line::styled(format!("● {ALL_OK}"), theme::fg(Tok::Ok))];
    }
    rows.into_iter()
        .map(|(name, reason)| {
            Line::from(vec![
                Span::styled(format!("● {name}"), theme::fg(Tok::Warn)),
                Span::styled(format!(" {} {reason}", fmt::DASH), theme::fg(Tok::Muted)),
            ])
        })
        .collect()
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
    fn snapshot_waiting() {
        let mut app = App::sample();
        app.last = None;
        let t = draw(&app, 100, 30);
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
    fn shortcut_list_has_no_card_key() {
        assert_eq!(KEYS.len(), 12);
        assert!(!KEYS.iter().any(|(k, _)| *k == "c"));
        let text = text(&draw(&App::sample(), 100, 30));
        assert!(!text.contains("kart değiştir"));
    }

    #[test]
    fn chip_pads_to_nine_plus_gap() {
        assert_eq!(chip("q"), "q         ");
        assert_eq!(chip("1-3").chars().count(), CHIP_W + 1);
        assert_eq!(chip("↑↓ j k"), "↑↓ j k    ");
        assert_eq!(chip("PgUp/PgDn"), "PgUp/PgDn ");
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
        // 60x20, 40x10 ve 20x4: örnekli ve örneksiz durumlar panik atmamalı.
        for (w, h) in [(60, 20), (40, 10), (20, 4)] {
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
