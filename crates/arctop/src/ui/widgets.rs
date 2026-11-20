//! Ortak saf widget yardımcıları (docs/TUI-DESIGN.md §1). `Frame` almaz;
//! widget ya da `Line` döndürür, çizimi çağıran yapar.
// Görünüm kartları (T-026..T-029) gelene dek kullanılmayanlar olabilir.
#![allow(dead_code)]

use arctop_core::history::Ring;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};

use crate::ui::theme::{self, Tok};

/// Yuvarlak kenarlıklı, `Surface` renkli panel; sol üstte kalın başlık, yatay iç boşluk 1.
pub fn panel<'a>(title: &'a str) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(Span::styled(format!(" {title} "), theme::bold(Tok::Text)))
        .padding(Padding::horizontal(1))
}

/// [`panel`] + sağ üstte ikincil başlık.
pub fn panel_with<'a>(title: &'a str, right: Line<'a>) -> Block<'a> {
    panel(title).title(right.right_aligned())
}

/// `width` hücrelik yatay çubuk: `━` dolu (`color`) + `─` boş (`Surface`).
/// `pct` 0..100 aralığına kırpılır (NaN → 0).
pub fn hbar(pct: f32, width: u16, color: Color) -> Line<'static> {
    let pct = if pct.is_nan() {
        0.0
    } else {
        pct.clamp(0.0, 100.0)
    };
    let w = usize::from(width);
    let filled = ((pct / 100.0) * w as f32).round() as usize;
    let filled = filled.min(w);
    Line::from(vec![
        Span::styled("━".repeat(filled), Style::default().fg(color)),
        Span::styled("─".repeat(w - filled), theme::fg(Tok::Surface)),
    ])
}

/// Kısayol çipleri: tuş `Accent` kalın, açıklama `Muted`; çipler arasında 2 boşluk.
pub fn key_chips(keys: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::with_capacity(keys.len() * 3);
    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled((*key).to_owned(), theme::bold(Tok::Accent)));
        if !desc.is_empty() {
            spans.push(Span::styled(format!(" {desc}"), theme::fg(Tok::Muted)));
        }
    }
    Line::from(spans)
}

/// Büyük değer: kalın metrik rengi + soluk birim.
pub fn big_value(value: &str, unit: &str, tok: Tok) -> Line<'static> {
    let mut spans = vec![Span::styled(value.to_owned(), theme::bold(tok))];
    if !unit.is_empty() {
        spans.push(Span::styled(format!(" {unit}"), theme::fg(Tok::Muted)));
    }
    Line::from(spans)
}

/// `Sparkline` verisi: son `width` nokta (eskiden yeniye), `None` → 0,
/// değerler ×10 yuvarlanır (negatif/NaN → 0).
pub fn spark_data(ring: &Ring<Option<f64>>, width: usize) -> Vec<u64> {
    let skip = ring.len().saturating_sub(width);
    ring.iter()
        .skip(skip)
        .map(|v| v.map_or(0, |x| (x * 10.0).round().max(0.0) as u64))
        .collect()
}

/// `area` içinde ortalanmış `w`×`h` dikdörtgen (alana sığacak şekilde kırpılır).
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// Ortalanmış soluk mesaj (boş liste / veri yok).
pub fn empty_state<'a>(msg: &'a str) -> Paragraph<'a> {
    Paragraph::new(msg)
        .alignment(Alignment::Center)
        .style(theme::fg(Tok::Muted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(l: &Line) -> String {
        l.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn hbar_fill_levels() {
        let c = Color::Green;
        assert_eq!(text(&hbar(0.0, 10, c)), "──────────");
        assert_eq!(text(&hbar(50.0, 10, c)), "━━━━━─────");
        assert_eq!(text(&hbar(100.0, 10, c)), "━━━━━━━━━━");
        assert_eq!(text(&hbar(250.0, 10, c)), "━━━━━━━━━━");
        assert_eq!(text(&hbar(-5.0, 4, c)), "────");
        assert_eq!(text(&hbar(f32::NAN, 4, c)), "────");
        assert_eq!(text(&hbar(50.0, 0, c)), "");
        assert_eq!(hbar(100.0, 3, c).spans[0].style.fg, Some(c));
    }

    fn ring(vals: &[Option<f64>]) -> Ring<Option<f64>> {
        let mut r = Ring::new(100);
        for v in vals {
            r.push(*v);
        }
        r
    }

    #[test]
    fn spark_data_cases() {
        assert_eq!(spark_data(&ring(&[]), 5), Vec::<u64>::new());
        assert_eq!(spark_data(&ring(&[None, Some(1.26)]), 5), [0, 13]);
        let long = ring(&[Some(1.0), Some(2.0), Some(3.0), None, Some(5.0)]);
        assert_eq!(spark_data(&long, 3), [30, 0, 50]);
        assert_eq!(spark_data(&long, 0), Vec::<u64>::new());
        assert_eq!(spark_data(&ring(&[Some(-4.0), Some(f64::NAN)]), 2), [0, 0]);
    }

    #[test]
    fn centered_rects() {
        let a = Rect::new(0, 0, 100, 30);
        assert_eq!(centered(a, 40, 10), Rect::new(30, 10, 40, 10));
        assert_eq!(centered(a, 200, 50), a);
        let b = Rect::new(5, 2, 21, 9);
        assert_eq!(centered(b, 10, 3), Rect::new(10, 5, 10, 3));
    }

    #[test]
    fn chips_and_big_value() {
        let l = key_chips(&[("q", "çıkış"), ("F1", "yardım")]);
        assert_eq!(text(&l), "q çıkış  F1 yardım");
        assert_eq!(text(&key_chips(&[])), "");
        let b = big_value("2.3", "GHz", Tok::Freq);
        assert_eq!(text(&b), "2.3 GHz");
        assert_eq!(b.spans[0].style.fg, Some(theme::color(Tok::Freq)));
    }

    #[test]
    fn panel_and_empty_state_render() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let mut t = Terminal::new(TestBackend::new(20, 5)).unwrap();
        t.draw(|f| {
            f.render_widget(panel_with("Başlık", Line::raw("sağ")), f.area());
            f.render_widget(empty_state("boş"), Rect::new(1, 2, 18, 1));
        })
        .unwrap();
        let s: String = t
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(s.contains("╭") && s.contains("Başlık") && s.contains("sağ") && s.contains("boş"));
    }
}
