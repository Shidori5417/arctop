//! Özet görünümü v2 (docs/TUI-DESIGN.md §4): dört stat kartı, Motorlar ve
//! En çok kullananlar panelleri ile altta Etkinlik grafiği. Tüm stil kararları `theme`/`widgets` üzerinden.

use arctop_core::history::Ring;
use arctop_core::{EngineClass, Metric, Snapshot};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols::{self, Marker};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Sparkline};
use std::time::Duration;

use crate::app::{App, ChartWindow};
use crate::fmt;
use crate::ui::theme::{self, Tok};
use crate::ui::widgets;

/// Motor/süreç satırında ad sütunu genişliği.
const NAME_W: u16 = 12;
/// Yüzde sütunu genişliği (`100.0%`).
const PCT_W: u16 = 6;
/// Süreç satırında VRAM sütunu genişliği.
const VRAM_W: u16 = 9;
/// Çubuğun çizilmesi için gereken en küçük genişlik.
const MIN_BAR_W: u16 = 3;
/// Sparkline'lı kart yüksekliği (kenarlık 2 + metin 2 + sparkline 2).
const CARD_H_FULL: u16 = 6;
/// Sparkline'sız kart yüksekliği (kenarlık 2 + metin 2).
const CARD_H_COMPACT: u16 = 4;
/// Bu yüksekliğin altında sparkline satırları düşer.
const SPARK_MIN_HEIGHT: u16 = 18;
/// Bu genişliğin altında kartlar 2x2 olur.
const WIDE_W: u16 = 100;
/// Bu genişliğin altında kartlar tek sütun olur ve süreç paneli düşer.
const NARROW_W: u16 = 60;
/// Listelenen en çok kullanan süreç sayısı.
const TOP_PROCS: usize = 5;
/// Alt sıranın en az içerik satırı (motor sınıfı sayısı).
const BOTTOM_MIN_ROWS: usize = 5;
/// Etkinlik paneli için gereken en küçük yükseklik.
const ACTIVITY_MIN_H: u16 = 5;
/// Sabit motor sırası.
const ENGINE_ORDER: [EngineClass; 5] = [
    EngineClass::Render,
    EngineClass::Copy,
    EngineClass::Video,
    EngineClass::VideoEnhance,
    EngineClass::Compute,
];
/// Sparkline değerleri `spark_data` içinde ×10 ölçeklenir.
const SPARK_SCALE: f64 = 10.0;

/// Görünümü `area` içine çizer.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = &app.last else {
        let msg = widgets::empty_state("örnek bekleniyor…");
        let y = area.y + area.height / 2;
        f.render_widget(
            msg,
            Rect {
                height: area.height.min(1),
                y,
                ..area
            },
        );
        return;
    };

    let spark = area.height >= SPARK_MIN_HEIGHT;
    let card_h = if spark { CARD_H_FULL } else { CARD_H_COMPACT };
    let cards = cards(app, snap);

    // Kart ızgarası: sütun sayısı genişliğe göre.
    let cols: usize = if area.width < NARROW_W {
        1
    } else if area.width < WIDE_W {
        2
    } else {
        4
    };
    let grid_rows = cards.len().div_ceil(cols);
    let grid_h = card_h * grid_rows as u16;
    let [grid, bottom] =
        Layout::vertical([Constraint::Length(grid_h), Constraint::Min(0)]).areas(area);
    let row_areas = Layout::vertical(vec![Constraint::Length(card_h); grid_rows]).split(grid);
    for (r, chunk) in cards.chunks(cols).enumerate() {
        let cells =
            Layout::horizontal(vec![Constraint::Ratio(1, cols as u32); cols]).split(row_areas[r]);
        for (card, cell) in chunk.iter().zip(cells.iter()) {
            draw_card(f, *cell, card, spark);
        }
    }

    // Alt sıra yüksekliği içerik kadar; altında yer kalırsa Etkinlik paneli.
    let row_h = (BOTTOM_MIN_ROWS.max(top_procs(snap).len()) as u16) + 2;
    let (row, activity) = if bottom.height >= row_h + ACTIVITY_MIN_H {
        let [r, a] =
            Layout::vertical([Constraint::Length(row_h), Constraint::Min(0)]).areas(bottom);
        (r, Some(a))
    } else {
        (bottom, None)
    };
    // Dar ekranda yalnızca Motorlar.
    if area.width < NARROW_W {
        draw_engines(f, row, snap);
    } else {
        let [l, r] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(row);
        draw_engines(f, l, snap);
        draw_procs(f, r, snap);
    }
    if let Some(a) = activity {
        draw_activity(f, a, app);
    }
}

/// Bir stat kartının içeriği.
struct Card {
    /// Panel başlığı.
    title: &'static str,
    /// Başlığın sağındaki ikincil bilgi.
    right: Line<'static>,
    /// Büyük değer satırı.
    value: Line<'static>,
    /// Bağlam satırı (muted).
    context: Line<'static>,
    /// Bağlam satırı sığmazsa kullanılan kısa biçim.
    context_short: Option<Line<'static>>,
    /// Sparkline serisi, rengi ve (varsa) ölçek üst sınırı.
    spark: Option<(Vec<f64>, Tok, Option<f64>)>,
}

/// Dört kartı (Frekans, Güç, Sıcaklık, VRAM) üretir.
fn cards(app: &App, snap: &Snapshot) -> Vec<Card> {
    vec![
        freq_card(app, snap),
        power_card(app, snap),
        temp_card(app, snap),
        vram_card(snap),
    ]
}

/// Ring'in tüm değerlerini ayrı bir vektöre alır (genişlik kırpması çizimde yapılır).
fn series(ring: &Ring<Option<f64>>) -> Vec<f64> {
    ring.iter().map(|v| v.unwrap_or(0.0)).collect()
}

/// Muted tek span'lı satır.
fn muted(text: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(text.into(), theme::fg(Tok::Muted)))
}

/// "—" muted.
fn dash() -> Line<'static> {
    muted(fmt::DASH)
}

/// `"2.3 GHz"` gibi metni (değer, birim) olarak ayırır.
fn split_unit(s: &str) -> (&str, &str) {
    s.split_once(' ').unwrap_or((s, ""))
}

/// Frekans değerini `fmt::freq` ile yazar; yoksa "—".
fn freq_text(m: &Metric<u32>) -> String {
    m.value()
        .map_or_else(|| fmt::DASH.to_string(), |&v| fmt::freq(v))
}

/// Frekans kartı.
fn freq_card(app: &App, snap: &Snapshot) -> Card {
    let fr = &snap.freq;
    let value = match fr.act.value() {
        None => dash(),
        Some(0) => muted("uykuda"),
        Some(&mhz) => {
            let text = fmt::freq(mhz);
            let (v, u) = split_unit(&text);
            widgets::big_value(v, u, Tok::Freq)
        }
    };
    let context = muted(format!(
        "cur {} · {}–{}",
        freq_text(&fr.cur),
        freq_text(&fr.min),
        freq_text(&fr.max)
    ));
    let short = |m: &Metric<u32>| {
        m.value()
            .map_or_else(|| fmt::DASH.to_string(), u32::to_string)
    };
    let context_short = muted(format!(
        "cur {} · {}–{}",
        short(&fr.cur),
        short(&fr.min),
        short(&fr.max)
    ));
    let right = fr
        .max
        .value()
        .map_or_else(Line::default, |&m| muted(format!(" maks {m} ")));
    Card {
        title: "Frekans",
        right,
        value,
        context,
        context_short: Some(context_short),
        spark: Some((
            series(app.history.freq_act()),
            Tok::Freq,
            fr.max.value().map(|&m| f64::from(m)),
        )),
    }
}

/// Güç kartı.
fn power_card(app: &App, snap: &Snapshot) -> Card {
    let value = match snap.power_w.value() {
        Some(w) => widgets::big_value(&format!("{w:.1}"), "W", Tok::Power),
        None => dash(),
    };
    let limit = snap.power_limit_w.value().copied();
    let context = match (snap.power_w.value(), limit) {
        (Some(&w), Some(l)) if l > 0.0 => muted(format!("%{:.0} limit", w / l * 100.0)),
        _ => dash(),
    };
    let right = limit.map_or_else(Line::default, |l| muted(format!(" limit {l:.0} W ")));
    Card {
        title: "Güç",
        right,
        value,
        context,
        context_short: None,
        spark: Some((
            series(app.history.power_w()),
            Tok::Power,
            limit.map(f64::from),
        )),
    }
}

/// Sıcaklık kartı (değer rengi eşiğe göre).
fn temp_card(app: &App, snap: &Snapshot) -> Card {
    let value = match snap.temp_c.value() {
        Some(&t) => {
            let style = Style::default()
                .fg(theme::temp_color(t))
                .add_modifier(ratatui::style::Modifier::BOLD);
            Line::from(vec![
                Span::styled(format!("{t:.0}"), style),
                Span::styled(" °C", theme::fg(Tok::Muted)),
            ])
        }
        None => dash(),
    };
    let context = match snap.voltage_mv.value() {
        Some(mv) => muted(format!("{mv} mV")),
        None => dash(),
    };
    let right = snap
        .fan_rpm
        .value()
        .map_or_else(Line::default, |rpm| muted(format!(" fan {rpm} ")));
    Card {
        title: "Sıcaklık",
        right,
        value,
        context,
        context_short: None,
        spark: Some((series(app.history.temp_c()), Tok::Temp, Some(100.0))),
    }
}

/// VRAM kartı: geçmiş serisi yok, sparkline çizilmez.
fn vram_card(snap: &Snapshot) -> Card {
    let value = match snap.vram_used.value() {
        Some(&b) => {
            let text = fmt::bytes(b);
            let (v, u) = split_unit(&text);
            widgets::big_value(v, u, Tok::Vram)
        }
        None => dash(),
    };
    let context = match (snap.vram_used.value(), snap.vram_total.value()) {
        (Some(&u), Some(&t)) if t > 0 => muted(format!(
            "%{:.0} / {}",
            u as f64 / t as f64 * 100.0,
            fmt::bytes(t)
        )),
        (Some(_), _) => muted("toplam bilinmiyor"),
        _ => dash(),
    };
    Card {
        title: "VRAM",
        right: Line::default(),
        value,
        context,
        context_short: None,
        spark: None,
    }
}

/// Kartı çizer; `spark` kapalıysa ya da yer yoksa sparkline atlanır.
fn draw_card(f: &mut Frame, area: Rect, card: &Card, spark: bool) {
    let block = widgets::panel_with(card.title, card.right.clone());
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height >= 1 {
        f.render_widget(
            Paragraph::new(card.value.clone()),
            Rect { height: 1, ..inner },
        );
    }
    if inner.height >= 2 {
        let r = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        let ctx = match &card.context_short {
            Some(short) if card.context.width() > usize::from(inner.width) => short,
            _ => &card.context,
        };
        f.render_widget(Paragraph::new(ctx.clone()), r);
    }
    let (true, Some((data, tok, max))) = (spark, &card.spark) else {
        return;
    };
    if inner.height < 3 || inner.width == 0 {
        return;
    }
    let r = Rect {
        y: inner.y + 2,
        height: (inner.height - 2).min(2),
        ..inner
    };
    let skip = data.len().saturating_sub(usize::from(r.width));
    let points: Vec<u64> = data[skip..]
        .iter()
        .map(|x| (x * SPARK_SCALE).round().max(0.0) as u64)
        .collect();
    let peak = points.iter().copied().max().unwrap_or(0);
    let top = max.map_or(peak, |m| ((m * SPARK_SCALE) as u64).max(peak));
    f.render_widget(
        Sparkline::default()
            .data(&points)
            .max(top.max(1))
            .bar_set(symbols::bar::NINE_LEVELS)
            .style(theme::fg(*tok)),
        r,
    );
}

/// Motor sınıfının 12 sütuna sığan etiketi.
fn engine_label(class: EngineClass) -> &'static str {
    match class {
        EngineClass::Render => "Render",
        EngineClass::Copy => "Copy",
        EngineClass::Video => "Video",
        EngineClass::VideoEnhance => "Video-Enh.",
        EngineClass::Compute => "Compute",
    }
}

/// Metni `w` karaktere kırpar, sağdan boşlukla doldurur.
fn fit(s: &str, w: u16) -> String {
    let w = usize::from(w);
    let t: String = s.chars().take(w).collect();
    format!("{t:<w$}")
}

/// Motorlar paneli.
fn draw_engines(f: &mut Frame, area: Rect, snap: &Snapshot) {
    let peak = snap
        .engines
        .iter()
        .map(|e| e.busy_pct)
        .fold(f32::NAN, f32::max);
    let right = if peak.is_nan() {
        Line::default()
    } else {
        muted(format!(" en yoğun %{peak:.0} "))
    };
    let block = widgets::panel_with("Motorlar", right);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    if snap.engines.is_empty() {
        let r = Rect {
            y: inner.y + inner.height / 2,
            height: 1,
            ..inner
        };
        f.render_widget(widgets::empty_state("motor verisi yok (ilk örnek)"), r);
        return;
    }
    let bar_w = inner.width.saturating_sub(NAME_W + PCT_W + 2);
    for (i, class) in ENGINE_ORDER
        .iter()
        .take(usize::from(inner.height))
        .enumerate()
    {
        let r = Rect {
            y: inner.y + i as u16,
            height: 1,
            ..inner
        };
        // Verisi olmayan sınıf 0% ve Muted çizilir.
        let found = snap.engines.iter().find(|e| e.class == *class);
        let pct = found.map_or(0.0, |e| e.busy_pct);
        let text_style = if found.is_some() {
            Style::default()
        } else {
            theme::fg(Tok::Muted)
        };
        let mut spans = vec![
            Span::styled(fit(engine_label(*class), NAME_W), text_style),
            Span::raw(" "),
        ];
        if bar_w >= MIN_BAR_W {
            spans.extend(widgets::hbar(pct, bar_w, theme::load_color(pct)).spans);
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("{:>w$}", fmt::pct(pct), w = usize::from(PCT_W)),
            text_style,
        ));
        f.render_widget(Paragraph::new(Line::from(spans)), r);
    }
}

/// Pencerenin nokta sayısı: `ceil(süre / aralık)` (charts.rs ile aynı kural).
fn window_points(window: ChartWindow, interval: Duration) -> usize {
    let secs = match window {
        ChartWindow::Last60s => 60.0,
        ChartWindow::Last5m => 300.0,
    };
    if interval.is_zero() {
        return 1;
    }
    (secs / interval.as_secs_f64()).ceil().max(1.0) as usize
}

/// Güç serisini 0–100'e normalize eder: `limit` (>0) varsa `w / limit × 100`, yoksa
/// serinin maksimumuna göre. Sonuç [0, 100]'e kırpılır; `None` noktalar korunur.
fn normalize_power(points: &[Option<f64>], limit: Option<f64>) -> Vec<Option<f64>> {
    let denom = match limit {
        Some(l) if l > 0.0 => l,
        _ => points.iter().flatten().copied().fold(0.0, f64::max),
    };
    points
        .iter()
        .map(|p| {
            p.map(|w| {
                if denom > 0.0 {
                    (w / denom * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            })
        })
        .collect()
}

/// `None` boşluklarında bölünmüş `(x, y)` parçaları; x, `offset` ile sağa hizalanır.
fn split_segments(points: &[Option<f64>], offset: usize) -> Vec<Vec<(f64, f64)>> {
    let mut segs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    for (i, p) in points.iter().enumerate() {
        match p {
            Some(v) => cur.push(((i + offset) as f64, v.clamp(0.0, 100.0))),
            None if !cur.is_empty() => segs.push(std::mem::take(&mut cur)),
            None => {}
        }
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    segs
}

/// Serinin pencereye düşen son noktaları (eskiden yeniye).
fn tail(ring: &Ring<Option<f64>>, n: usize) -> Vec<Option<f64>> {
    ring.iter().copied().rev().take(n).rev().collect()
}

/// Tek parçalık çizgi veri kümesi.
fn line_ds(pts: &[(f64, f64)], tok: Tok) -> Dataset<'_> {
    Dataset::default()
        .data(pts)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(theme::fg(tok))
}

/// Etkinlik paneli: en yoğun motor % ve normalize güç, aynı grafikte.
fn draw_activity(f: &mut Frame, area: Rect, app: &App) {
    let win = match app.window {
        ChartWindow::Last60s => "60 sn",
        ChartWindow::Last5m => "5 dk",
    };
    let right = muted(format!(" {win} · en yoğun motor / güç "));
    let legend = Line::from(vec![
        Span::raw(" "),
        Span::styled("━ motor", theme::fg(Tok::Busy)),
        Span::raw("  "),
        Span::styled("━ güç", theme::fg(Tok::Power)),
        Span::raw(" "),
    ])
    .centered();
    let block = widgets::panel_with("Etkinlik", right).title_bottom(legend);

    let span = window_points(app.window, app.interval);
    let busy = tail(app.history.busy_total(), span);
    let limit = app
        .last
        .as_ref()
        .and_then(|s| s.power_limit_w.value().copied())
        .map(f64::from);
    let power = normalize_power(&tail(app.history.power_w(), span), limit);
    let busy_segs = split_segments(&busy, span.saturating_sub(busy.len()));
    let power_segs = split_segments(&power, span.saturating_sub(power.len()));
    if busy_segs.is_empty() && power_segs.is_empty() {
        f.render_widget(widgets::empty_state("veri toplanıyor…").block(block), area);
        return;
    }
    let muted_s = theme::fg(Tok::Muted);
    let x_axis = Axis::default()
        .bounds([0.0, span.saturating_sub(1).max(1) as f64])
        .style(theme::fg(Tok::Surface))
        .labels([
            Line::default(),
            Line::default(),
            Line::styled("şimdi", muted_s),
        ]);
    let y_axis = Axis::default()
        .bounds([0.0, 100.0])
        .style(theme::fg(Tok::Surface))
        .labels([
            Line::styled("0%", muted_s),
            Line::styled("50%", muted_s),
            Line::styled("100%", muted_s),
        ]);
    let datasets: Vec<Dataset<'_>> = power_segs
        .iter()
        .map(|p| line_ds(p, Tok::Power))
        .chain(busy_segs.iter().map(|p| line_ds(p, Tok::Busy)))
        .collect();
    f.render_widget(
        Chart::new(datasets)
            .block(block)
            .x_axis(x_axis)
            .y_axis(y_axis),
        area,
    );
}

/// Bir sürecin en yüksek motor doluluğu (motor yoksa 0).
fn proc_peak(p: &arctop_core::ProcUsage) -> f32 {
    p.engines.iter().map(|e| e.busy_pct).fold(0.0, f32::max)
}

/// En yüksek motor yüzdesine göre ilk `TOP_PROCS` süreç (App sıralamasından bağımsız).
fn top_procs(snap: &Snapshot) -> Vec<&arctop_core::ProcUsage> {
    let mut v: Vec<_> = snap.procs.iter().collect();
    v.sort_by(|a, b| {
        proc_peak(b)
            .total_cmp(&proc_peak(a))
            .then(a.pid.cmp(&b.pid))
    });
    v.truncate(TOP_PROCS);
    v
}

/// En çok kullananlar paneli.
fn draw_procs(f: &mut Frame, area: Rect, snap: &Snapshot) {
    let block = widgets::panel("En çok kullananlar");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let procs = top_procs(snap);
    if procs.is_empty() {
        let r = Rect {
            y: inner.y + inner.height / 2,
            height: 1,
            ..inner
        };
        f.render_widget(widgets::empty_state("süreç yok"), r);
        return;
    }
    let bar_w = inner.width.saturating_sub(NAME_W + PCT_W + VRAM_W + 3);
    for (i, p) in procs.iter().take(usize::from(inner.height)).enumerate() {
        let r = Rect {
            y: inner.y + i as u16,
            height: 1,
            ..inner
        };
        let pct = proc_peak(p);
        let mut spans = vec![
            Span::raw(fit(&p.comm, NAME_W)),
            Span::raw(format!(
                "{:>w$}",
                format!("{pct:.0}%"),
                w = usize::from(PCT_W)
            )),
            Span::raw(" "),
        ];
        if bar_w >= MIN_BAR_W {
            spans.extend(widgets::hbar(pct, bar_w, theme::load_color(pct)).spans);
            spans.push(Span::raw(" "));
        }
        let (vram, style) = match p.vram_bytes.value() {
            Some(&b) => (fmt::bytes(b), Style::default()),
            None => (fmt::DASH.to_string(), theme::fg(Tok::Muted)),
        };
        spans.push(Span::styled(
            format!("{vram:>w$}", w = usize::from(VRAM_W)),
            style,
        ));
        f.render_widget(Paragraph::new(Line::from(spans)), r);
    }
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
    fn snapshot_80x24() {
        let app = App::sample();
        // unwrap: yalnızca testlerde (AGENTS.md).
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn fits_80x24() {
        let text = screen(&App::sample(), 80, 24);
        for want in [
            "Frekans",
            "Güç",
            "Sıcaklık",
            "VRAM",
            "Motorlar",
            "En çok kullananlar",
            "Render",
            "Video",
            "blender",
            "en yoğun",
            "8.0 GiB",
        ] {
            assert!(text.contains(want), "80x24 ekranda yok: {want}");
        }
    }

    #[test]
    fn narrow_hides_procs_panel() {
        let text = screen(&App::sample(), 50, 40);
        assert!(text.contains("Motorlar"));
        assert!(!text.contains("En çok kullananlar"));
    }

    #[test]
    fn no_sample_waits_without_panic() {
        let mut app = App::sample();
        app.last = None;
        assert!(screen(&app, 80, 24).contains("örnek bekleniyor…"));
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
    fn top_procs_sorted_by_peak_engine() {
        let app = App::sample();
        let Some(s) = app.last.as_ref() else {
            unreachable!("App::sample her zaman dolu snapshot tutar");
        };
        let names: Vec<_> = top_procs(s).iter().map(|p| p.comm.as_str()).collect();
        assert_eq!(names, ["blender", "mpv", "firefox", "steam", "Xorg"]);
    }

    #[test]
    fn normalize_power_cases() {
        let pts = [Some(50.0), None, Some(100.0), Some(300.0)];
        // Limitli: w/limit*100, 100'e kırpılır.
        assert_eq!(
            normalize_power(&pts, Some(200.0)),
            [Some(25.0), None, Some(50.0), Some(100.0)]
        );
        // Limitsiz: serinin maksimumuna göre.
        let r = normalize_power(&pts, None);
        assert!((r[0].unwrap() - 50.0 / 3.0).abs() < 1e-9);
        assert_eq!(r[1], None);
        assert!((r[2].unwrap() - 100.0 / 3.0).abs() < 1e-9);
        assert_eq!(r[3], Some(100.0));
        // Sıfır limit yok sayılır; hepsi None ya da 0 ise sıfıra bölme yok.
        assert_eq!(normalize_power(&[None, None], Some(0.0)), [None, None]);
        assert_eq!(normalize_power(&[Some(0.0)], None), [Some(0.0)]);
    }

    #[test]
    fn all_engine_classes_listed() {
        let text = screen(&App::sample(), 100, 30);
        for want in [
            "Render",
            "Copy",
            "Video ",
            "Video-Enh.",
            "Compute",
            "Etkinlik",
        ] {
            assert!(text.contains(want), "yok: {want}");
        }
    }

    #[test]
    fn snapshot_120x40() {
        let app = App::sample();
        // unwrap: yalnızca testlerde (AGENTS.md).
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn small_sizes_render_without_panic() {
        let mut app = App::sample();
        for (w, h) in [
            (120, 40),
            (100, 30),
            (80, 24),
            (60, 20),
            (40, 10),
            (20, 4),
            (1, 1),
            (0, 0),
        ] {
            let _ = screen(&app, w, h);
        }
        app.last = None;
        for (w, h) in [(80, 24), (20, 4), (0, 0)] {
            let _ = screen(&app, w, h);
        }
    }
}
