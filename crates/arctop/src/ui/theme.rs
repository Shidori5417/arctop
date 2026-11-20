//! Renk teması: semantik tokenlar ve üç renk modu (docs/TUI-DESIGN.md §2).
//! Tüm renk ve stil kararları yalnızca burada verilir; görünümlerde `Color::` yasaktır.
// Görünüm kartları (T-026..T-029) gelene dek kullanılmayanlar olabilir.
#![allow(dead_code)]

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

/// Renk modu; süreç başında bir kez seçilir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// 24 bit RGB.
    TrueColor,
    /// 256 renkli indeksli palet.
    Ansi256,
    /// Renksiz: yalnızca bold/reverse/dim.
    NoColor,
}

static MODE: OnceLock<ColorMode> = OnceLock::new();

/// Renk modunu ayarlar (ilk çağrı kazanır). Çağrılmazsa `TrueColor` kullanılır,
/// böylece testler ortamdan bağımsız ve deterministik olur.
pub fn init(mode: ColorMode) {
    let _ = MODE.set(mode);
}

fn mode() -> ColorMode {
    MODE.get().copied().unwrap_or(ColorMode::TrueColor)
}

/// Ortam değişkenlerinden mod seçer (`NO_COLOR`, `COLORTERM`).
pub fn detect_mode() -> ColorMode {
    mode_from(
        std::env::var("NO_COLOR").ok().as_deref(),
        std::env::var("COLORTERM").ok().as_deref(),
    )
}

/// Saf mod seçimi: boş olmayan `NO_COLOR` → renksiz; `COLORTERM` `truecolor`/`24bit`
/// → truecolor; aksi halde 256 renk.
pub fn mode_from(no_color: Option<&str>, colorterm: Option<&str>) -> ColorMode {
    if no_color.is_some_and(|v| !v.is_empty()) {
        return ColorMode::NoColor;
    }
    match colorterm {
        Some(c) if c.eq_ignore_ascii_case("truecolor") || c.eq_ignore_ascii_case("24bit") => {
            ColorMode::TrueColor
        }
        _ => ColorMode::Ansi256,
    }
}

/// Semantik renk tokenı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tok {
    /// Seçim, odak (açık Arc mavisi).
    Accent,
    /// Rozet ve aktif sekme zemini.
    AccentBg,
    /// Normal metin.
    Text,
    /// İkincil metin, birimler.
    Muted,
    /// Kenarlık, boş çubuk.
    Surface,
    /// Zebra satır zemini.
    RowAlt,
    /// Düşük yük.
    Ok,
    /// Orta yük.
    Warn,
    /// Yüksek yük.
    Crit,
    /// Frekans serisi.
    Freq,
    /// Güç serisi.
    Power,
    /// Sıcaklık serisi.
    Temp,
    /// Motor doluluğu serisi.
    Busy,
    /// VRAM serisi.
    Vram,
}

/// Tokenın geçerli moddaki rengi; `NoColor` modunda her zaman `Color::Reset`.
pub fn color(t: Tok) -> Color {
    color_in(mode(), t)
}

/// Mod verilen saf renk tablosu (§2).
fn color_in(mode: ColorMode, t: Tok) -> Color {
    use Tok::*;
    let rgb = Color::Rgb;
    let ix = Color::Indexed;
    match mode {
        ColorMode::NoColor => Color::Reset,
        ColorMode::TrueColor => match t {
            Accent => rgb(0x3D, 0x9B, 0xFF),
            AccentBg => rgb(0x00, 0x71, 0xC5),
            Text => Color::Reset,
            Muted => rgb(0x8B, 0x93, 0xA7),
            Surface => rgb(0x3A, 0x40, 0x50),
            RowAlt => rgb(0x1C, 0x20, 0x28),
            Ok => rgb(0x4A, 0xDE, 0x80),
            Warn => rgb(0xFB, 0xBF, 0x24),
            Crit => rgb(0xF8, 0x71, 0x71),
            Freq => rgb(0x60, 0xA5, 0xFA),
            Power => rgb(0xFB, 0xBF, 0x24),
            Temp => rgb(0xFB, 0x92, 0x3C),
            Busy => rgb(0xA7, 0x8B, 0xFA),
            Vram => rgb(0x2D, 0xD4, 0xBF),
        },
        ColorMode::Ansi256 => match t {
            Accent => ix(75),
            AccentBg => ix(25),
            Text => Color::Reset,
            Muted => ix(246),
            Surface => ix(238),
            RowAlt => ix(234),
            Ok => ix(114),
            Warn => ix(220),
            Crit => ix(203),
            Freq => ix(75),
            Power => ix(220),
            Temp => ix(209),
            Busy => ix(141),
            Vram => ix(79),
        },
    }
}

/// Token rengiyle ön plan stili.
pub fn fg(t: Tok) -> Style {
    Style::default().fg(color(t))
}

/// Token rengiyle kalın ön plan stili.
pub fn bold(t: Tok) -> Style {
    fg(t).add_modifier(Modifier::BOLD)
}

/// Yüke göre renk: <50 `Ok`, <80 `Warn`, aksi halde `Crit`.
pub fn load_color(pct: f32) -> Color {
    color(if pct < 50.0 {
        Tok::Ok
    } else if pct < 80.0 {
        Tok::Warn
    } else {
        Tok::Crit
    })
}

/// Sıcaklığa (°C) göre renk: <60 `Ok`, <80 `Warn`, aksi halde `Crit`.
pub fn temp_color(c: f32) -> Color {
    color(if c < 60.0 {
        Tok::Ok
    } else if c < 80.0 {
        Tok::Warn
    } else {
        Tok::Crit
    })
}

/// Seçili satır stili: `AccentBg` zemin + bold (renksizde ters video).
pub fn selected() -> Style {
    if mode() == ColorMode::NoColor {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default()
            .bg(color(Tok::AccentBg))
            .add_modifier(Modifier::BOLD)
    }
}

/// Normal panel kenarlığı (`Surface`).
pub fn border() -> Style {
    fg(Tok::Surface)
}

/// Odaktaki panel kenarlığı (`Accent`).
pub fn border_focus() -> Style {
    fg(Tok::Accent)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Tok; 14] = [
        Tok::Accent,
        Tok::AccentBg,
        Tok::Text,
        Tok::Muted,
        Tok::Surface,
        Tok::RowAlt,
        Tok::Ok,
        Tok::Warn,
        Tok::Crit,
        Tok::Freq,
        Tok::Power,
        Tok::Temp,
        Tok::Busy,
        Tok::Vram,
    ];

    #[test]
    fn truecolor_palette() {
        assert_eq!(
            color_in(ColorMode::TrueColor, Tok::Accent),
            Color::Rgb(0x3D, 0x9B, 0xFF)
        );
        assert_eq!(color_in(ColorMode::TrueColor, Tok::Text), Color::Reset);
        // Test sürecinde init çağrılmaz → varsayılan TrueColor.
        assert_eq!(color(Tok::Ok), Color::Rgb(0x4A, 0xDE, 0x80));
    }

    #[test]
    fn ansi256_palette_is_indexed() {
        assert_eq!(
            color_in(ColorMode::Ansi256, Tok::Accent),
            Color::Indexed(75)
        );
        assert_eq!(
            color_in(ColorMode::Ansi256, Tok::Surface),
            Color::Indexed(238)
        );
        assert_eq!(color_in(ColorMode::Ansi256, Tok::Text), Color::Reset);
    }

    #[test]
    fn no_color_is_all_reset() {
        for t in ALL {
            assert_eq!(color_in(ColorMode::NoColor, t), Color::Reset, "{t:?}");
        }
    }

    #[test]
    fn mode_detection_rules() {
        assert_eq!(mode_from(Some("1"), Some("truecolor")), ColorMode::NoColor);
        assert_eq!(mode_from(Some(""), Some("truecolor")), ColorMode::TrueColor);
        assert_eq!(mode_from(None, Some("24bit")), ColorMode::TrueColor);
        assert_eq!(mode_from(None, Some("TrueColor")), ColorMode::TrueColor);
        assert_eq!(mode_from(None, Some("")), ColorMode::Ansi256);
        assert_eq!(mode_from(None, None), ColorMode::Ansi256);
    }

    #[test]
    fn load_and_temp_thresholds() {
        assert_eq!(load_color(0.0), color(Tok::Ok));
        assert_eq!(load_color(49.9), color(Tok::Ok));
        assert_eq!(load_color(50.0), color(Tok::Warn));
        assert_eq!(load_color(79.9), color(Tok::Warn));
        assert_eq!(load_color(80.0), color(Tok::Crit));
        assert_eq!(temp_color(59.9), color(Tok::Ok));
        assert_eq!(temp_color(60.0), color(Tok::Warn));
        assert_eq!(temp_color(80.0), color(Tok::Crit));
    }

    #[test]
    fn styles() {
        assert_eq!(selected().bg, Some(color(Tok::AccentBg)));
        assert!(selected().add_modifier.contains(Modifier::BOLD));
        assert_eq!(border().fg, Some(color(Tok::Surface)));
        assert_eq!(border_focus().fg, Some(color(Tok::Accent)));
        assert!(bold(Tok::Ok).add_modifier.contains(Modifier::BOLD));
    }
}
