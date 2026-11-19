//! Grafik görünümü: frekans, güç, sıcaklık ve toplam doluluk zaman serileri (§6).
//!
//! `g` ile seçilen pencere (60 sn / 5 dk) `App::interval` kullanılarak nokta
//! sayısına çevrilir (`window_len`) ve her serinin kuyruğu ratatui `Chart`'ına
//! (`GraphType::Line`, `Marker::Braille`) verilir. `None` noktalar çizilmez;
//! çizilecek nokta kalmazsa blok "veri yok" gösterir.

use std::time::Duration;

use arctop_core::history::Ring;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};

use crate::app::{App, ChartWindow};
use crate::fmt;
use crate::ui::theme;

/// Tek bir gösterge bloğunun tanımı.
struct ChartDef<'a> {
    /// Blok başlığındaki metrik adı (ör. "Frekans (MHz)").
    title: &'static str,
    /// Son değer için birim (ör. "MHz").
    unit: &'static str,
    /// Değerlerin ondalık basamak sayısı (frekans 0, güç 1).
    decimals: usize,
    /// Seri kaynağı.
    series: &'a Ring<Option<f64>>,
    /// Sabit y üst sınırı (sıcaklık/doluluk için 100); `None` = seri maksimumu, en az 1.
    y_max_fixed: Option<f64>,
}

/// `ChartDef` kurucusu; alan sırasını tek yerde tutar.
fn chart_def<'a>(
    title: &'static str,
    unit: &'static str,
    decimals: usize,
    series: &'a Ring<Option<f64>>,
    y_max_fixed: Option<f64>,
) -> ChartDef<'a> {
    ChartDef {
        title,
        unit,
        decimals,
        series,
        y_max_fixed,
    }
}

/// Görünümü `area` içine çizer: 2x2 ızgara (frekans, güç, sıcaklık, doluluk).
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if app.last.is_none() {
        render_waiting(f, area);
        return;
    }
    let [top, bottom] = Layout::vertical([Constraint::Percentage(50); 2]).areas(area);
    let [freq, power] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(top);
    let [temp, busy] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(bottom);
    let defs = [
        (
            freq,
            chart_def("Frekans (MHz)", "MHz", 0, app.history.freq_act(), None),
        ),
        (
            power,
            chart_def("Güç (W)", "W", 1, app.history.power_w(), None),
        ),
        (
            temp,
            chart_def("Sıcaklık (°C)", "°C", 0, app.history.temp_c(), Some(100.0)),
        ),
        (
            busy,
            chart_def(
                "En yoğun motor (%)",
                "%",
                0,
                app.history.busy_total(),
                Some(100.0),
            ),
        ),
    ];
    for (chart_area, def) in defs {
        render_chart(f, chart_area, app, &def);
    }
}

/// Henüz snapshot gelmemişse bekleme mesajı çizer (panik yok).
fn render_waiting(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title(" Grafikler ")
        .title_style(theme::title());
    f.render_widget(
        Paragraph::new("örnek bekleniyor…")
            .style(theme::dim())
            .alignment(Alignment::Center)
            .block(block),
        area,
    );
}

/// Tek bir gösterge bloğu çizer: başlıkta pencere ve son değer, içeride seri.
fn render_chart(f: &mut Frame, area: Rect, app: &App, def: &ChartDef) {
    let points = window_tail(
        def.series,
        window_len(app.window, app.interval, def.series.len()),
    );
    let segs = segments(&points);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title(title_line(app, def));

    if segs.is_empty() {
        f.render_widget(
            Paragraph::new("veri yok").style(theme::dim()).block(block),
            area,
        );
        return;
    }

    let y_max = match def.y_max_fixed {
        Some(fixed) => fixed,
        None => segs.iter().flatten().map(|p| p.1).fold(1.0, f64::max),
    };
    let x_max = (points.len().saturating_sub(1)).max(1) as f64;
    let y_axis = Axis::default()
        .bounds([0.0, y_max])
        .style(theme::dim())
        .labels([
            Line::styled(num(0.0, def.decimals), theme::dim()),
            Line::styled(num(y_max / 2.0, def.decimals), theme::dim()),
            Line::styled(num(y_max, def.decimals), theme::dim()),
        ]);
    let x_axis = Axis::default()
        .bounds([0.0, x_max])
        .style(theme::dim())
        .labels([
            Line::styled("0", theme::dim()),
            Line::styled(num(x_max / 2.0, 0), theme::dim()),
            Line::styled("şimdi", theme::dim()),
        ]);
    let datasets: Vec<Dataset<'_>> = segs
        .iter()
        .map(|pts| {
            Dataset::default()
                .data(pts)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(theme::ACCENT))
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

/// Blok başlık satırı: " Frekans (MHz) · 60 sn · 2280 MHz".
/// Son değer okunamadıysa solak "—"; seri tamamen boşsa değer kısmı yoktur.
fn title_line(app: &App, def: &ChartDef) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!(" {} · {}", def.title, window_label(app.window)),
        theme::title(),
    )];
    if let Some(last) = def.series.last() {
        let (text, style) = match last {
            Some(v) => (
                format!(" · {:.*} {}", def.decimals, v, def.unit),
                theme::title(),
            ),
            None => (format!(" · {}", fmt::DASH), theme::dim()),
        };
        spans.push(Span::styled(text, style));
    }
    Line::from(spans)
}

/// Başlıkta gösterilecek pencere adı.
fn window_label(window: ChartWindow) -> &'static str {
    match window {
        ChartWindow::Last60s => "60 sn",
        ChartWindow::Last5m => "5 dk",
    }
}

/// Pencerenin kapsadığı nokta sayısı: `ceil(pencere_süresi / aralık)`,
/// `len` (halka tampon uzunluğu) ile sınırlı.
///
/// App aralığı [250 ms, 5 s] aralığında tutulduğundan sonuç her zaman sonli ve
/// ≥ 12'dir; yine de fonksiyon toplam kalsın diye NaN/sonsuz değerlerde
/// `f64::max` ve doyurucu `as usize` dönüşümü güvenli kalır.
fn window_len(window: ChartWindow, interval: Duration, len: usize) -> usize {
    let secs = match window {
        ChartWindow::Last60s => 60.0,
        ChartWindow::Last5m => 300.0,
    };
    if interval.is_zero() {
        return len;
    }
    let pts = (secs / interval.as_secs_f64()).ceil().max(1.0) as usize;
    pts.min(len)
}

/// Serinin son `n` noktası (eskiden yeniye sırayla).
fn window_tail(series: &Ring<Option<f64>>, n: usize) -> Vec<Option<f64>> {
    series.iter().copied().rev().take(n).rev().collect()
}

/// Seriyi `None` boşluklarında ardışık parçalara böler; her nokta `(indeks, değer)`
/// çiftidir. Böylece çizgi okunamayan aralıkların üzerinden atlamaz.
fn segments(points: &[Option<f64>]) -> Vec<Vec<(f64, f64)>> {
    let mut segs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    for (i, p) in points.iter().enumerate() {
        if let Some(v) = p {
            cur.push((i as f64, *v));
        } else if !cur.is_empty() {
            segs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    segs
}

/// Eksen etiketi ve başlık değeri için sayı biçimi: `decimals` ondalık basamak.
fn num(v: f64, decimals: usize) -> String {
    format!("{v:.decimals$}")
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
            segments(&pts),
            vec![
                vec![(0.0, 1.0), (1.0, 2.0)],
                vec![(3.0, 4.0)],
                vec![(6.0, 7.0)],
            ]
        );
        assert!(segments(&[]).is_empty());
        assert!(segments(&[None, None]).is_empty());
    }

    #[test]
    fn renders_all_blocks_at_80x24() {
        let text = draw(&App::sample(), 80, 24);
        for want in [
            "Frekans (MHz)",
            "Güç (W)",
            "Sıcaklık (°C)",
            "En yoğun motor (%)",
            "60 sn",
            "şimdi",
        ] {
            assert!(text.contains(want), "ekranda yok: {want}");
        }
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
        assert!(!text.contains("Frekans (MHz)"));
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
    fn charts_snapshot() {
        let app = App::sample();
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }
}
