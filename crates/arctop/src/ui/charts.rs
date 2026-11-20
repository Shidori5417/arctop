//! Grafik görünümü v2: frekans, güç, sıcaklık ve toplam doluluk zaman serileri
//! (TUI-DESIGN §5).
//!
//! Üstte tek satır pencere seçici, altında 2x2 (dar/kısa ekranda 1x4) panel ızgarası.
//! `g` ile seçilen pencere (60 sn / 5 dk) `App::interval` ile nokta sayısına çevrilir
//! (`window_len`); seri x ekseninde sağa hizalanır, böylece `-60 sn … şimdi` etiketleri
//! geçmiş pencereden kısa olsa da doğru kalır. `None` noktalar çizgiyi böler.

use std::time::Duration;

use arctop_core::history::Ring;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

use crate::app::{App, ChartWindow};
use crate::fmt;
use crate::ui::theme::{self, Tok};
use crate::ui::widgets;

/// Metrik türü: biçimi ve rengi belirler.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// Frekans (MHz; `fmt::freq`).
    Freq,
    /// Güç (W).
    Power,
    /// Sıcaklık (°C).
    Temp,
    /// Doluluk (%).
    Busy,
}

impl Kind {
    /// Serinin renk tokenı.
    fn tok(self) -> Tok {
        match self {
            Kind::Freq => Tok::Freq,
            Kind::Power => Tok::Power,
            Kind::Temp => Tok::Temp,
            Kind::Busy => Tok::Busy,
        }
    }
}

/// Değeri birimiyle yazar (y etiketi, son değer, min/ort/maks için ortak).
fn fmt_val(kind: Kind, v: f64) -> String {
    match kind {
        Kind::Freq => fmt::freq(v.max(0.0).round() as u32),
        Kind::Power => format!("{v:.1} W"),
        Kind::Temp => format!("{v:.0} °C"),
        Kind::Busy => format!("{v:.0} %"),
    }
}

/// Tek bir gösterge panelinin tanımı.
struct ChartDef<'a> {
    /// Panel başlığındaki metrik adı.
    title: &'static str,
    /// Metrik türü.
    kind: Kind,
    /// Seri kaynağı.
    series: &'a Ring<Option<f64>>,
    /// Sabit y üst sınırı (sıcaklık/doluluk için 100); `None` = seri maksimumu, en az 1.
    y_max_fixed: Option<f64>,
}

/// Görünümü `area` içine çizer: pencere seçici + panel ızgarası.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.last.is_none() {
        f.render_widget(widgets::empty_state("örnek bekleniyor…"), area);
        return;
    }
    let [sel, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    f.render_widget(window_selector(app.window), sel);

    let defs = [
        ChartDef {
            title: "Frekans",
            kind: Kind::Freq,
            series: app.history.freq_act(),
            y_max_fixed: None,
        },
        ChartDef {
            title: "Güç",
            kind: Kind::Power,
            series: app.history.power_w(),
            y_max_fixed: None,
        },
        ChartDef {
            title: "Sıcaklık",
            kind: Kind::Temp,
            series: app.history.temp_c(),
            y_max_fixed: Some(100.0),
        },
        ChartDef {
            title: "En yoğun motor",
            kind: Kind::Busy,
            series: app.history.busy_total(),
            y_max_fixed: Some(100.0),
        },
    ];
    for (rect, def) in grid(body).into_iter().zip(defs.iter()) {
        // Panel çerçevesi + en az bir iç satır sığmıyorsa atla (panik yok).
        if rect.width >= 6 && rect.height >= 3 {
            render_chart(f, rect, app, def);
        }
    }
}

/// Gövdeyi dört panele böler: genişlik >= 90 ve yükseklik >= 20 ise 2x2, değilse 1x4.
fn grid(body: Rect) -> [Rect; 4] {
    if body.width >= 90 && body.height >= 19 {
        let [top, bottom] = Layout::vertical([Constraint::Ratio(1, 2); 2]).areas(body);
        let [a, b] = Layout::horizontal([Constraint::Ratio(1, 2); 2]).areas(top);
        let [c, d] = Layout::horizontal([Constraint::Ratio(1, 2); 2]).areas(bottom);
        [a, b, c, d]
    } else {
        Layout::vertical([Constraint::Ratio(1, 4); 4]).areas(body)
    }
}

/// `Pencere: 60 sn  5 dk` satırı; aktif hap `theme::selected()`.
fn window_selector(active: ChartWindow) -> Line<'static> {
    let mut spans = vec![Span::styled("Pencere: ", theme::fg(Tok::Muted))];
    for w in [ChartWindow::Last60s, ChartWindow::Last5m] {
        let style = if w == active {
            theme::selected()
        } else {
            theme::fg(Tok::Muted)
        };
        spans.push(Span::styled(format!(" {} ", window_label(w)), style));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

/// Tek bir panel çizer: sağ üstte son değer, altta min · ort · maks, içeride seri.
fn render_chart(f: &mut Frame, area: Rect, app: &App, def: &ChartDef) {
    let span = window_span(app.window, app.interval);
    let points = window_tail(
        def.series,
        window_len(app.window, app.interval, def.series.len()),
    );
    let color = theme::color(def.kind.tok());
    let right = match def.series.last() {
        Some(Some(v)) => Line::styled(
            format!(" {} ", fmt_val(def.kind, *v)),
            theme::bold(def.kind.tok()),
        ),
        Some(None) => Line::styled(format!(" {} ", fmt::DASH), theme::fg(Tok::Muted)),
        None => Line::default(),
    };
    let mut block = widgets::panel_with(def.title, right);
    if let Some((mn, avg, mx)) = stats(&points) {
        block = block.title_bottom(
            Line::styled(
                format!(
                    " {} · {} · {} ",
                    fmt_val(def.kind, mn),
                    fmt_val(def.kind, avg),
                    fmt_val(def.kind, mx)
                ),
                theme::fg(Tok::Muted),
            )
            .centered(),
        );
    }

    // Sağa hizalama: veri pencereden kısaysa solda boşluk kalır.
    let offset = span.saturating_sub(points.len());
    let segs = segments(&points, offset);
    if segs.is_empty() {
        f.render_widget(widgets::empty_state("veri yok").block(block), area);
        return;
    }

    let y_max = match def.y_max_fixed {
        Some(fixed) => fixed,
        None => segs.iter().flatten().map(|p| p.1).fold(1.0, f64::max),
    };
    let x_max = span.saturating_sub(1).max(1) as f64;
    let muted = theme::fg(Tok::Muted);
    let y_axis = Axis::default()
        .bounds([0.0, y_max])
        .style(theme::fg(Tok::Surface))
        .labels([
            Line::styled(fmt_val(def.kind, 0.0), muted),
            Line::styled(fmt_val(def.kind, y_max / 2.0), muted),
            Line::styled(fmt_val(def.kind, y_max), muted),
        ]);
    let [l0, l1, l2] = x_labels(app.window);
    let x_axis = Axis::default()
        .bounds([0.0, x_max])
        .style(theme::fg(Tok::Surface))
        .labels([
            Line::styled(l0, muted),
            Line::styled(l1, muted),
            Line::styled(l2, muted),
        ]);
    let datasets: Vec<Dataset<'_>> = segs
        .iter()
        .map(|pts| {
            Dataset::default()
                .data(pts)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(color))
        })
        .collect();
    f.render_widget(
        Chart::new(datasets)
            .block(block)
            .x_axis(x_axis)
            .y_axis(y_axis),
        area,
    );
}

/// `(min, ortalama, maks)` — yalnızca `Some` noktalardan; hiç yoksa `None`.
fn stats(points: &[Option<f64>]) -> Option<(f64, f64, f64)> {
    let vals: Vec<f64> = points.iter().flatten().copied().collect();
    if vals.is_empty() {
        return None;
    }
    let mn = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let avg = vals.iter().sum::<f64>() / vals.len() as f64;
    Some((mn, avg, mx))
}

/// Pencere adı.
fn window_label(window: ChartWindow) -> &'static str {
    match window {
        ChartWindow::Last60s => "60 sn",
        ChartWindow::Last5m => "5 dk",
    }
}

/// x ekseni etiketleri (sol, orta, sağ); saf.
fn x_labels(window: ChartWindow) -> [&'static str; 3] {
    match window {
        ChartWindow::Last60s => ["-60 sn", "-30 sn", "şimdi"],
        ChartWindow::Last5m => ["-5 dk", "-2.5 dk", "şimdi"],
    }
}

/// Pencerenin kapsadığı nokta sayısı (geçmişten bağımsız): `ceil(süre / aralık)`.
/// Aralık sıfırsa 1 döner (App aralığı zaten [250 ms, 5 s]).
fn window_span(window: ChartWindow, interval: Duration) -> usize {
    let secs = match window {
        ChartWindow::Last60s => 60.0,
        ChartWindow::Last5m => 300.0,
    };
    if interval.is_zero() {
        return 1;
    }
    (secs / interval.as_secs_f64()).ceil().max(1.0) as usize
}

/// Pencerenin kapsadığı nokta sayısı, `len` (halka tampon uzunluğu) ile sınırlı.
fn window_len(window: ChartWindow, interval: Duration, len: usize) -> usize {
    if interval.is_zero() {
        return len;
    }
    window_span(window, interval).min(len)
}

/// Serinin son `n` noktası (eskiden yeniye sırayla).
fn window_tail(series: &Ring<Option<f64>>, n: usize) -> Vec<Option<f64>> {
    series.iter().copied().rev().take(n).rev().collect()
}

/// Seriyi `None` boşluklarında ardışık parçalara böler; her nokta `(indeks + offset, değer)`
/// çiftidir. Böylece çizgi okunamayan aralıkların üzerinden atlamaz.
fn segments(points: &[Option<f64>], offset: usize) -> Vec<Vec<(f64, f64)>> {
    let mut segs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    for (i, p) in points.iter().enumerate() {
        if let Some(v) = p {
            cur.push(((i + offset) as f64, *v));
        } else if !cur.is_empty() {
            segs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::DASH;
    use arctop_core::history::History;
    use arctop_core::{Device, Driver, Snapshot};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::time::Instant;

    /// Çizip ekrandaki metni döndürür (TestBackend hata üretmez; testlerde unwrap bilinçli).
    fn draw(app: &App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| render(f, f.area(), app)).unwrap();
        t.backend().to_string()
    }

    /// Tüm metrikleri okunamayan tek noktalı App (gerçek "yok" durumu).
    fn unavailable_app() -> App {
        let device = Device {
            card: 0,
            pci_slot: "0000:00:00.0".into(),
            device_id: 0,
            name: "test".into(),
            driver: Driver::I915,
        };
        let mut app = App::new(device, Vec::new(), Duration::from_secs(1));
        app.on_snapshot(Snapshot::empty(Instant::now()));
        app
    }

    #[test]
    fn window_len_examples() {
        let s = Duration::from_secs(1);
        assert_eq!(window_len(ChartWindow::Last60s, s, 300), 60);
        assert_eq!(window_len(ChartWindow::Last5m, s, 300), 300);
        let ms = Duration::from_millis(250);
        assert_eq!(window_len(ChartWindow::Last60s, ms, 5000), 240);
        assert_eq!(window_len(ChartWindow::Last5m, ms, 5000), 1200);
        // Halka tampon sınırı: kapasite hesaptan küçükse kırpılır.
        assert_eq!(window_len(ChartWindow::Last5m, ms, 300), 300);
        assert_eq!(
            window_len(ChartWindow::Last60s, Duration::from_secs(5), 10),
            10
        );
        assert_eq!(window_len(ChartWindow::Last5m, s, 0), 0);
    }

    #[test]
    fn window_tail_takes_last_n() {
        let mut r = Ring::<Option<f64>>::new(5);
        for i in 0..5 {
            r.push(Some(i as f64));
        }
        assert_eq!(window_tail(&r, 3), vec![Some(2.0), Some(3.0), Some(4.0)]);
        assert_eq!(
            window_tail(&r, 10),
            vec![Some(0.0), Some(1.0), Some(2.0), Some(3.0), Some(4.0)]
        );
        assert!(window_tail(&r, 0).is_empty());
    }

    #[test]
    fn segments_split_at_gaps() {
        let pts = [Some(1.0), Some(2.0), None, Some(4.0), None, None, Some(7.0)];
        assert_eq!(
            segments(&pts, 0),
            vec![
                vec![(0.0, 1.0), (1.0, 2.0)],
                vec![(3.0, 4.0)],
                vec![(6.0, 7.0)],
            ]
        );
        assert!(segments(&[], 0).is_empty());
        assert!(segments(&[None, None], 0).is_empty());
        assert_eq!(segments(&[Some(1.0)], 5), vec![vec![(5.0, 1.0)]]);
    }

    #[test]
    fn renders_all_blocks_at_80x24() {
        let text = draw(&App::sample(), 80, 24);
        for want in [
            "Frekans",
            "Güç",
            "Sıcaklık",
            "En yoğun motor",
            "60 sn",
            "şimdi",
        ] {
            assert!(text.contains(want), "ekranda yok: {want}");
        }
    }

    #[test]
    fn x_labels_pure() {
        assert_eq!(
            x_labels(ChartWindow::Last60s),
            ["-60 sn", "-30 sn", "şimdi"]
        );
        assert_eq!(x_labels(ChartWindow::Last5m), ["-5 dk", "-2.5 dk", "şimdi"]);
    }

    #[test]
    fn stats_and_fmt() {
        assert_eq!(stats(&[None, None]), None);
        assert_eq!(stats(&[Some(1.0), None, Some(3.0)]), Some((1.0, 2.0, 3.0)));
        assert_eq!(fmt_val(Kind::Freq, 2400.0), "2.4 GHz");
        assert_eq!(fmt_val(Kind::Freq, 850.0), "850 MHz");
        assert_eq!(fmt_val(Kind::Power, 12.34), "12.3 W");
        assert_eq!(fmt_val(Kind::Temp, 65.0), "65 °C");
        assert_eq!(fmt_val(Kind::Busy, 100.0), "100 %");
    }

    #[test]
    fn grid_layouts() {
        let wide = grid(Rect::new(0, 0, 100, 29));
        assert_eq!(wide[0].y, wide[1].y);
        let narrow = grid(Rect::new(0, 0, 80, 23));
        assert!(narrow[1].y > narrow[0].y);
    }

    #[test]
    fn five_minute_window_label() {
        let mut app = App::sample();
        app.window = ChartWindow::Last5m;
        let text = draw(&app, 80, 24);
        assert!(text.contains("5 dk"));
    }

    #[test]
    fn waits_when_no_snapshot() {
        let mut app = App::sample();
        app.last = None;
        app.history = History::new();
        let text = draw(&app, 80, 24);
        assert!(text.contains("örnek bekleniyor"));
        assert!(!text.contains("Frekans"));
    }

    #[test]
    fn unavailable_series_shows_veri_yok() {
        let text = draw(&unavailable_app(), 80, 24);
        assert!(text.contains("veri yok"));
        assert!(text.contains(DASH), "son değer yerine — görünmeli");
    }

    #[test]
    fn no_panic_at_small_sizes() {
        // 40x10 ve 20x4: veri var, veri yok ve örnek bekleniyor durumları panik atmamalı.
        for (w, h) in [(40, 10), (20, 4)] {
            draw(&App::sample(), w, h);
            draw(&unavailable_app(), w, h);
            let mut waiting = App::sample();
            waiting.last = None;
            waiting.history = History::new();
            draw(&waiting, w, h);
        }
    }

    #[test]
    fn no_panic_60x20() {
        draw(&App::sample(), 60, 20);
        draw(&unavailable_app(), 60, 20);
    }

    #[test]
    fn charts_snapshot_5m() {
        let mut app = App::sample();
        app.window = ChartWindow::Last5m;
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn charts_snapshot() {
        let app = App::sample();
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }
}
