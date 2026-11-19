//! Yardım görünümü: tuş kısayolları (§6).
//! Yer tutucu: asıl içerik sonraki görev kartında gelir.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::theme;

/// Görünümü `area` içine çizer.
pub fn render(f: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title_style(theme::title())
        .title(" Yardım ");
    f.render_widget(
        Paragraph::new("yakında").style(theme::dim()).block(block),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn placeholder_renders() {
        let app = App::sample();
        let mut t = Terminal::new(TestBackend::new(40, 5)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        let text: String = t
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("yakında"));
    }
}
