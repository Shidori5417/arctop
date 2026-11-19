//! TUI: görünümlerin ve renk temasının bildirimi (§6).

mod charts;
mod help;
mod overview;
mod procs;
pub mod theme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Tabs};

use crate::app::{App, View};

/// Tüm ekranı çizer: üstte sekmeler + kart adı, ortada aktif görünüm, altta kısayol satırı.
pub fn draw(f: &mut Frame, app: &App) {
    let [top, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_tabs(f, top, app);
    match app.view {
        View::Overview => overview::render(f, body, app),
        View::Charts => charts::render(f, body, app),
        View::Procs => procs::render(f, body, app),
        View::Help => help::render(f, body, app),
    }
    draw_footer(f, bottom, app);
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
    let name = format!(" {} ", app.device.name);
    let name_w = name.chars().count() as u16;
    let [tabs_area, name_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(name_w)]).areas(area);
    // Help sekme satırında yer almaz: o sırada hiçbir sekme seçili görünmez.
    let selected = match app.view {
        View::Overview => Some(0),
        View::Charts => Some(1),
        View::Procs => Some(2),
        View::Help => None,
    };
    let tabs = Tabs::new(["Özet", "Grafikler", "Süreçler"])
        .select(selected)
        .style(theme::tab())
        .highlight_style(theme::tab_selected());
    f.render_widget(tabs, tabs_area);
    f.render_widget(Paragraph::new(name).style(theme::title()), name_area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let line = if app.filter_editing {
        Line::from(vec![
            Span::styled("/", theme::title()),
            Span::raw(app.filter.clone()),
            Span::styled("▏  Enter uygula · Esc temizle", theme::dim()),
        ])
    } else {
        let filter = if app.filter.is_empty() {
            String::new()
        } else {
            format!(" · filtre: {}", app.filter)
        };
        Line::from(Span::styled(
            format!(
                "1-3/Tab görünüm · s sıra · / filtre · +/- {}ms · g süre · F1 yardım · q çıkış{filter}",
                app.interval.as_millis()
            ),
            theme::dim(),
        ))
    };
    f.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn screen(app: &App) -> String {
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
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
        for want in ["Özet", "Grafikler", "Süreçler", "Intel Arc A750", "q çıkış"] {
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
        assert!(screen(&app).contains("/fire"));
    }
}
