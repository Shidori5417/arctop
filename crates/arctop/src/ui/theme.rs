//! Renk teması: Arc mavisi vurgu, terminal temasıyla uyumlu (§6).
//! Tüm renk ve stiller yalnızca burada tanımlanır.
// Görünüm kartları (T-010..T-013) gelene dek kullanılmayanlar olabilir.
#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

/// Arc mavisi vurgu rengi.
pub const ACCENT: Color = Color::Rgb(0x00, 0x71, 0xc5);
/// Gösterge düşük seviye rengi.
pub const GAUGE_LOW: Color = Color::Green;
/// Gösterge orta seviye rengi.
pub const GAUGE_MID: Color = Color::Yellow;
/// Gösterge yüksek seviye rengi.
pub const GAUGE_HIGH: Color = Color::Red;
/// Orta seviye eşiği (%).
pub const MID_THRESHOLD: f32 = 50.0;
/// Yüksek seviye eşiği (%).
pub const HIGH_THRESHOLD: f32 = 80.0;

/// Çerçeve stili.
pub fn border() -> Style {
    Style::default().fg(ACCENT)
}

/// Başlık stili.
pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Soluk stil (Unavailable "—", ipuçları).
pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Yüzdeye göre gösterge rengi (düşük/orta/yüksek eşikleri).
pub fn gauge_color(pct: f32) -> Color {
    if pct >= HIGH_THRESHOLD {
        GAUGE_HIGH
    } else if pct >= MID_THRESHOLD {
        GAUGE_MID
    } else {
        GAUGE_LOW
    }
}

/// Yüzdeye göre gösterge stili.
pub fn gauge(pct: f32) -> Style {
    Style::default().fg(gauge_color(pct))
}

/// Sekme satırı normal stili.
pub fn tab() -> Style {
    Style::default().fg(Color::Gray)
}

/// Seçili sekme stili.
pub fn tab_selected() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_thresholds() {
        assert_eq!(gauge_color(0.0), GAUGE_LOW);
        assert_eq!(gauge_color(49.9), GAUGE_LOW);
        assert_eq!(gauge_color(50.0), GAUGE_MID);
        assert_eq!(gauge_color(80.0), GAUGE_HIGH);
    }
}
