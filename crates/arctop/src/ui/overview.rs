//! Özet görünümü (§6): kart bilgisi, metrik göstergeleri ve motor çubukları.

use std::fmt::Display;

use arctop_core::{EngineClass, FreqInfo, Metric, Snapshot};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::app::App;
use crate::fmt;
use crate::ui::theme::{self, Tok};

/// Satır etiketi sütununun genişliği (en uzun etiket: "Video-Enhance").
const LABEL_W: u16 = 13;
/// Göstergenin çizilmesi için gereken en küçük genişlik.
const MIN_GAUGE_W: u16 = 3;
/// Yüzde ölçeği (0–100).
const PCT_SCALE: f64 = 100.0;

/// Satır içeriği: sağdaki değer metni ve (varsa) gösterge (oran, stil).
type RowContent = (Line<'static>, Option<(f64, Style)>);

/// Görünümü `area` içine çizer.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title_style(theme::bold(Tok::Accent))
        .title(" Özet ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Üst satır: kart adı, sürücü, PCI yuvası, örnekleme aralığı.
    if let Some(row) = row_at(inner, inner.y) {
        f.render_widget(Paragraph::new(header_line(app)), row);
    }

    let Some(snap) = &app.last else {
        // Henüz örnek yok: bekleme yazısı, panik yok.
        if let Some(row) = row_at(inner, inner.y.saturating_add(2)) {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "örnek bekleniyor…",
                    theme::fg(Tok::Muted),
                ))),
                row,
            );
        }
        return;
    };

    // Metrik satırları.
    let mut y = inner.y.saturating_add(2); // üst satır + boş satır
    for (label, (text, gauge)) in [
        ("Frekans", freq_content(&snap.freq)),
        ("Güç", power_content(snap)),
        ("Sıcaklık", temp_content(snap)),
        ("Fan", plain_content(&snap.fan_rpm, "RPM")),
        ("Voltaj", plain_content(&snap.voltage_mv, "mV")),
        ("VRAM", vram_content(snap)),
    ] {
        let Some(row) = row_at(inner, y) else {
            return; // Alan bitti; kalan satırlar sığmaz.
        };
        draw_row(f, row, label, text, gauge);
        y = y.saturating_add(1);
    }

    // Motor çubukları.
    y = y.saturating_add(1); // boş satır
    if snap.engines.is_empty() {
        if let Some(row) = row_at(inner, y) {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "motor verisi yok (ilk örnek)",
                    theme::fg(Tok::Muted),
                ))),
                row,
            );
        }
        return;
    }
    for e in &snap.engines {
        let Some(row) = row_at(inner, y) else {
            return; // Alan bitti; kalan motorlar sığmaz.
        };
        let r = ratio(f64::from(e.busy_pct), PCT_SCALE);
        draw_row(
            f,
            row,
            engine_label(e.class),
            Line::from(Span::raw(fmt::pct(e.busy_pct))),
            Some((r, Style::default().fg(theme::load_color(e.busy_pct)))),
        );
        y = y.saturating_add(1);
    }
}

/// Üst bilgi satırı: kart adı, sürücü, PCI yuvası, örnekleme aralığı.
fn header_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled(app.device.name.clone(), theme::bold(Tok::Accent)),
        Span::raw(format!(
            " · {} · {} · ",
            app.device.driver, app.device.pci_slot
        )),
        Span::styled(
            format!("aralık: {} ms", app.interval.as_millis()),
            theme::fg(Tok::Muted),
        ),
    ])
}

/// Frekans satırı: gösterge act/max, yanında `cur` ve `min`–`max` metni.
///
/// `act == 0` geçerli bir değerdir: GPU uykudadır, "uykuda" etiketi gösterilir.
fn freq_content(freq: &FreqInfo) -> RowContent {
    let Some(&act) = freq.act.value() else {
        return (dash_line(), None);
    };
    let act_text = if act == 0 {
        "uykuda".to_string()
    } else {
        fmt::metric(&freq.act, "MHz")
    };
    let line = Line::from(vec![
        Span::raw(act_text),
        Span::raw(" · cur "),
        value_span(&freq.cur, ""),
        Span::raw(" · "),
        value_span(&freq.min, ""),
        Span::raw("–"),
        value_span(&freq.max, "MHz"),
    ]);
    let gauge = freq.max.value().map(|&max| {
        let r = ratio(f64::from(act), f64::from(max));
        (
            r,
            Style::default().fg(theme::load_color((r * PCT_SCALE) as f32)),
        )
    });
    (line, gauge)
}

/// Güç satırı: gösterge power_w/power_limit_w.
fn power_content(snap: &Snapshot) -> RowContent {
    let (Some(&pw), limit) = (snap.power_w.value(), snap.power_limit_w.value()) else {
        return (dash_line(), None);
    };
    let line = Line::from(vec![
        Span::raw(fmt::metric(&snap.power_w, "W")),
        Span::raw(" / "),
        value_span(&snap.power_limit_w, "W"),
    ]);
    let gauge = limit.map(|&pl| {
        let r = ratio(f64::from(pw), f64::from(pl));
        (
            r,
            Style::default().fg(theme::load_color((r * PCT_SCALE) as f32)),
        )
    });
    (line, gauge)
}

/// Sıcaklık satırı: 0–100 ölçek; eşik renkleri sıcaklık değerinden.
fn temp_content(snap: &Snapshot) -> RowContent {
    let Some(&t) = snap.temp_c.value() else {
        return (dash_line(), None);
    };
    (
        Line::from(Span::raw(fmt::metric(&snap.temp_c, "°C"))),
        Some((
            ratio(f64::from(t), PCT_SCALE),
            Style::default().fg(theme::load_color(t)),
        )),
    )
}

/// Ölçeği olmayan düz metrik satırı (fan, voltaj).
fn plain_content<T: Display>(m: &Metric<T>, unit: &str) -> RowContent {
    match m {
        Metric::Value(_) => (Line::from(Span::raw(fmt::metric(m, unit))), None),
        Metric::Unavailable(_) => (dash_line(), None),
    }
}

/// VRAM satırı: vram_used; toplam yoksa yalnızca kullanılan miktar.
fn vram_content(snap: &Snapshot) -> RowContent {
    let Some(&used) = snap.vram_used.value() else {
        return (dash_line(), None);
    };
    match snap.vram_total.value() {
        Some(&total) => {
            let r = ratio(used as f64, total as f64);
            let line = Line::from(vec![
                Span::raw(fmt::bytes(used)),
                Span::raw(" / "),
                Span::raw(fmt::bytes(total)),
            ]);
            (
                line,
                Some((
                    r,
                    Style::default().fg(theme::load_color((r * PCT_SCALE) as f32)),
                )),
            )
        }
        None => (
            Line::from(vec![
                Span::raw(fmt::bytes(used)),
                Span::raw(" · toplam: "),
                Span::styled(fmt::DASH, theme::fg(Tok::Muted)),
            ]),
            None,
        ),
    }
}

/// Unavailable satır içeriği: dim "—". Nedeni F1 ekranında yazılır, burada yazılmaz.
fn dash_line() -> Line<'static> {
    Line::from(Span::styled(fmt::DASH, theme::fg(Tok::Muted)))
}

/// Metrik değerini düz metin; Unavailable ise dim "—".
fn value_span<T: Display>(m: &Metric<T>, unit: &str) -> Span<'static> {
    match m {
        Metric::Value(_) => Span::raw(fmt::metric(m, unit)),
        Metric::Unavailable(_) => Span::styled(fmt::DASH, theme::fg(Tok::Muted)),
    }
}

/// Motor sınıfının satır etiketi.
fn engine_label(class: EngineClass) -> &'static str {
    match class {
        EngineClass::Render => "Render",
        EngineClass::Copy => "Copy",
        EngineClass::Video => "Video",
        EngineClass::VideoEnhance => "Video-Enhance",
        EngineClass::Compute => "Compute",
    }
}

/// `a / b` oranını 0–1 aralığına kısıtlar; `b` geçersizse 0 döner.
fn ratio(a: f64, b: f64) -> f64 {
    if b > 0.0 && a.is_finite() {
        (a / b).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// `inner` içindeki `y`. satır; sığmıyorsa `None`.
fn row_at(inner: Rect, y: u16) -> Option<Rect> {
    if y < inner.bottom() {
        Some(Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        })
    } else {
        None
    }
}

/// Tek satırlık metrik satırı: solda etiket, ortada gösterge (varsa), sağda değer.
fn draw_row(f: &mut Frame, row: Rect, label: &str, text: Line, gauge: Option<(f64, Style)>) {
    let label_w = LABEL_W.min(row.width);
    f.render_widget(
        Paragraph::new(label),
        Rect {
            x: row.x,
            y: row.y,
            width: label_w,
            height: 1,
        },
    );

    // Metin sağa dayalı; sığmayan kısım kırpılır.
    // `width()` bu ratatui sürümünde `usize` döndürür; satır metinleri kısa olduğu için
    // u16'ya dönüştürme güvenlidir.
    let text_w = (text.width() as u16).min(row.width.saturating_sub(label_w));
    if text_w == 0 {
        return;
    }
    let text_x = row.x + row.width - text_w;
    if let Some((r, style)) = gauge {
        // Gösterge etiket ile metin arasında yer varsa çizilir.
        let gauge_w = text_x.saturating_sub(row.x + label_w + 1);
        if gauge_w >= MIN_GAUGE_W {
            f.render_widget(
                Gauge::default().ratio(r).gauge_style(style),
                Rect {
                    x: row.x + label_w,
                    y: row.y,
                    width: gauge_w,
                    height: 1,
                },
            );
        }
    }
    f.render_widget(
        Paragraph::new(text).alignment(Alignment::Right),
        Rect {
            x: text_x,
            y: row.y,
            width: text_w,
            height: 1,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// `render`'ı `w`×`h` arka planda çizip ekran metnini döndürür.
    fn screen(app: &App, w: u16, h: u16) -> String {
        // unwrap: yalnızca testlerde (AGENTS.md).
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| render(f, f.area(), app)).unwrap();
        t.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn overview_snapshot() {
        let app = App::sample();
        // unwrap: yalnızca testlerde (AGENTS.md).
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn fits_80x24() {
        let text = screen(&App::sample(), 80, 24);
        for want in [
            "Intel Arc A750",
            "i915",
            "0000:03:00.0",
            "1000 ms",
            "Frekans",
            "Güç",
            "Sıcaklık",
            "Fan",
            "Voltaj",
            "VRAM",
            "Render",
            "Video",
            "—", // fan Unavailable
        ] {
            assert!(text.contains(want), "80x24 ekranda yok: {want}");
        }
    }

    #[test]
    fn no_sample_waits_without_panic() {
        let mut app = App::sample();
        app.last = None;
        let text = screen(&app, 80, 24);
        assert!(text.contains("örnek bekleniyor…"));
    }

    #[test]
    fn zero_freq_shows_sleep_label() {
        let mut app = App::sample();
        let Some(s) = app.last.as_mut() else {
            unreachable!("App::sample her zaman dolu snapshot tutar");
        };
        s.freq.act = Metric::Value(0);
        assert!(screen(&app, 80, 24).contains("uykuda"));
    }

    #[test]
    fn empty_engines_note() {
        let mut app = App::sample();
        let Some(s) = app.last.as_mut() else {
            unreachable!("App::sample her zaman dolu snapshot tutar");
        };
        s.engines.clear();
        assert!(screen(&app, 80, 24).contains("motor verisi yok (ilk örnek)"));
    }

    #[test]
    fn tiny_area_renders_without_panic() {
        let _ = screen(&App::sample(), 20, 4);
        let mut app = App::sample();
        app.last = None;
        let _ = screen(&app, 20, 4);
    }
}
