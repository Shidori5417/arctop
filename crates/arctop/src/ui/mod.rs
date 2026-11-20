//! TUI çerçevesi: başlık satırı, gövde, altbilgi ve modal yardım (docs/TUI-DESIGN.md §3).

mod charts;
mod help;
mod overview;
mod procs;
pub mod theme;
pub mod widgets;

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, View};
use theme::Tok;

/// Bu genişliğin altında çerçeve kısaltılır (cihaz/sürücü atılır, çip açıklamaları düşer).
const NARROW_W: u16 = 70;

/// Tüm ekranı çizer: üstte başlık satırı, ortada aktif görünüm, altta kısayol çipleri.
pub fn draw(f: &mut Frame, app: &App) {
    draw_at(f, app, Instant::now());
}

/// [`draw`]'ın deterministik biçimi: canlılık noktası `now`'a göre hesaplanır.
pub fn draw_at(f: &mut Frame, app: &App, now: Instant) {
    let [top, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_header(f, top, app, now);
    match app.view {
        // Yardım modaldır: önce arkadaki görünüm, sonra üstüne pencere.
        View::Help => {
            render_view(f, body, app, app.help_return_view());
            help::render(f, body, app);
        }
        v => render_view(f, body, app, v),
    }
    draw_footer(f, bottom, app);
}

/// Verilen görünümü gövdeye çizer.
fn render_view(f: &mut Frame, body: Rect, app: &App, view: View) {
    match view {
        View::Overview => overview::render(f, body, app),
        View::Charts => charts::render(f, body, app),
        View::Procs => procs::render(f, body, app),
        // Help kendi kendinin arka planı olamaz (help_return_view hiç Help dönmez).
        View::Help => {}
    }
}

/// Aralığı kısa yazar: 1 sn ve üstü "1.0 s", altı "250 ms".
fn interval_text(d: Duration) -> String {
    if d >= Duration::from_secs(1) {
        format!("{:.1} s", d.as_secs_f32())
    } else {
        format!("{} ms", d.as_millis())
    }
}

fn spans_width(spans: &[Span]) -> u16 {
    let w: usize = spans.iter().map(Span::width).sum();
    u16::try_from(w).unwrap_or(u16::MAX)
}

fn draw_header(f: &mut Frame, area: Rect, app: &App, now: Instant) {
    let narrow = area.width < NARROW_W;
    let active = match app.view {
        View::Help => app.help_return_view(),
        v => v,
    };
    let mut left = vec![
        Span::styled(" ◆ arctop ", theme::selected()),
        Span::raw(" "),
    ];
    for (i, (view, name)) in [
        (View::Overview, "Özet"),
        (View::Charts, "Grafikler"),
        (View::Procs, "Süreçler"),
    ]
    .into_iter()
    .enumerate()
    {
        let n = i + 1;
        if view == active {
            left.push(Span::styled(format!(" {n} {name} "), theme::selected()));
        } else if narrow {
            left.push(Span::styled(format!(" {n} "), theme::fg(Tok::Muted)));
        } else {
            left.push(Span::styled(format!(" {n} {name} "), theme::fg(Tok::Muted)));
        }
    }

    let dot = if app.is_fresh(now) {
        theme::fg(Tok::Ok)
    } else {
        theme::fg(Tok::Surface)
    };
    let tail = |name: Option<String>| -> Vec<Span<'static>> {
        let mut v = Vec::new();
        if let Some(n) = name {
            v.push(Span::styled(n, theme::fg(Tok::Muted)));
            v.push(Span::raw("  "));
        }
        v.push(Span::styled("●", dot));
        v.push(Span::styled(
            format!(" {} ", interval_text(app.interval)),
            theme::fg(Tok::Muted),
        ));
        v
    };
    // Sığan ilk seçenek: cihaz·sürücü → yalnızca cihaz → yalnızca nokta+aralık.
    let left_w = spans_width(&left);
    let right = [
        tail(Some(format!("{} · {}", app.device.name, app.device.driver))),
        tail(Some(app.device.name.clone())),
        tail(None),
    ]
    .into_iter()
    .skip(usize::from(narrow))
    .find(|r| left_w + 1 + spans_width(r) <= area.width);

    f.render_widget(Paragraph::new(Line::from(left)), area);
    if let Some(r) = right {
        f.render_widget(
            Paragraph::new(Line::from(r)).alignment(Alignment::Right),
            area,
        );
    }
}

/// Görünüme göre bağlamsal kısayol çipleri.
fn footer_keys(view: View) -> Vec<(&'static str, &'static str)> {
    match view {
        View::Overview => vec![
            ("1-3", "görünüm"),
            ("+/-", "aralık"),
            ("F1", "yardım"),
            ("q", "çıkış"),
        ],
        View::Charts => vec![
            ("1-3", "görünüm"),
            ("g", "60 sn/5 dk"),
            ("+/-", "aralık"),
            ("F1", "yardım"),
            ("q", "çıkış"),
        ],
        View::Procs => vec![
            ("1-3", "görünüm"),
            ("↑↓", "seç"),
            ("s", "sırala"),
            ("/", "filtre"),
            ("+/-", "aralık"),
            ("F1", "yardım"),
            ("q", "çıkış"),
        ],
        View::Help => vec![("F1/Esc", "kapat"), ("q", "çıkış")],
    }
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let line = if app.filter_editing {
        Line::from(vec![
            Span::styled(" /", theme::bold(Tok::Accent)),
            Span::raw(format!(" {}", app.filter)),
            Span::styled("▏  Enter uygula · Esc temizle", theme::fg(Tok::Muted)),
        ])
    } else {
        let mut keys = footer_keys(app.view);
        if area.width < NARROW_W {
            // Dar ekran: açıklamaları at, yalnızca tuşlar kalsın.
            keys.iter_mut().for_each(|k| k.1 = "");
        }
        let mut line = widgets::key_chips(&keys);
        line.spans.insert(0, Span::raw(" "));
        if !app.filter.is_empty() && app.view != View::Help {
            line.spans.push(Span::styled(
                format!("  filtre: {}", app.filter),
                theme::fg(Tok::Muted).add_modifier(Modifier::ITALIC),
            ));
        }
        line
    };
    f.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Canlılık noktası testlerde deterministik: son snapshot'tan 100 ms sonrası.
    fn fixed_now(app: &App) -> Instant {
        app.last_snapshot_at.unwrap_or_else(Instant::now) + Duration::from_millis(100)
    }

    fn screen(app: &App) -> String {
        screen_sized(app, 80, 24)
    }

    fn screen_sized(app: &App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw_at(f, app, fixed_now(app))).unwrap();
        t.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn draw_smoke_shows_tabs_and_device() {
        let text = screen(&App::sample());
        for want in [
            "Özet",
            "Grafikler",
            "Süreçler",
            "Intel Arc A750",
            "i915",
            "q çıkış",
        ] {
            assert!(text.contains(want), "ekranda yok: {want}");
        }
    }

    #[test]
    fn draw_all_views_and_filter_footer() {
        let mut app = App::sample();
        for v in [View::Charts, View::Procs, View::Help] {
            app.view = v;
            screen(&app);
        }
        app.filter_editing = true;
        app.filter = "fire".into();
        assert!(screen(&app).contains("/ fire"));
    }

    #[test]
    fn snapshot_overview_100x30() {
        let app = App::sample();
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| draw_at(f, &app, fixed_now(&app))).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn no_panic_at_small_sizes_all_views() {
        let mut app = App::sample();
        for v in [View::Overview, View::Charts, View::Procs, View::Help] {
            app.view = v;
            for (w, h) in [(80, 24), (40, 10), (20, 4), (10, 2), (1, 1)] {
                screen_sized(&app, w, h);
            }
        }
        app.filter_editing = true;
        screen_sized(&app, 20, 4);
        app.last = None;
        app.view = View::Procs;
        screen_sized(&app, 40, 10);
    }

    #[test]
    fn narrow_header_drops_driver_and_chip_descriptions() {
        let app = App::sample();
        let wide = screen_sized(&app, 100, 24);
        let head = |t: &str, w: usize| t.chars().take(w).collect::<String>();
        let wide_head = head(&wide, 100);
        assert!(wide_head.contains("i915") && wide_head.contains("1.0 s"));
        let narrow = screen_sized(&app, 60, 24);
        assert!(!head(&narrow, 60).contains("i915"), "başlıkta sürücü yok");
        assert!(!narrow.contains("çıkış"));
        assert!(narrow.contains('●'));
    }

    #[test]
    fn help_is_modal_over_previous_view() {
        let mut app = App::sample();
        app.view = View::Procs;
        app.on_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::F(1),
        ));
        let text = screen_sized(&app, 100, 30);
        assert!(text.contains("Yardım") && text.contains("Kısayollar"));
        assert!(text.contains("Süreçler"), "arkadaki görünüm çizilmeli");
    }

    #[test]
    fn liveness_dot_color_follows_freshness() {
        let app = App::sample();
        let t0 = app.last_snapshot_at.unwrap();
        let dot_fg = |now| {
            let mut t = Terminal::new(TestBackend::new(100, 5)).unwrap();
            t.draw(|f| draw_at(f, &app, now)).unwrap();
            let buf = t.backend().buffer().clone();
            buf.content().iter().find(|c| c.symbol() == "●").unwrap().fg
        };
        assert_eq!(dot_fg(t0), theme::color(Tok::Ok));
        assert_eq!(
            dot_fg(t0 + Duration::from_secs(2)),
            theme::color(Tok::Surface)
        );
    }
}
