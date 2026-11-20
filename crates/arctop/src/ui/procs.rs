//! Süreç görünümü v2: GPU kullanan süreçler tablosu, seçim, filtre ve sıralama (TUI-DESIGN §6).

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};

use arctop_core::{EngineClass, Metric, ProcUsage};

use crate::app::{App, SortKey};
use crate::fmt;
use crate::ui::theme::{self, Tok};
use crate::ui::widgets;

/// Tablodaki motor sütunlarının sırası (geniş düzende gösterilir).
const ENGINE_COLUMNS: [(&str, EngineClass); 5] = [
    ("Render", EngineClass::Render),
    ("Copy", EngineClass::Copy),
    ("Video", EngineClass::Video),
    ("VE", EngineClass::VideoEnhance),
    ("Compute", EngineClass::Compute),
];

/// Bu genişliğin (dış alan) altında motor sütunları gizlenir.
const WIDE_MIN: u16 = 90;
/// GPU sütunundaki mini çubuk genişliği.
const GPU_BAR: u16 = 10;
/// GPU sütunu genişliği: çubuk + boşluk + `100.0%`.
const GPU_W: u16 = GPU_BAR + 1 + 6;

/// Görünümü `area` içine çizer: tek panel içinde süreç tablosu.
///
/// Satır sırası `App::visible_procs` tarafından belirlenir (filtre + sıralama
/// orada yapılmıştır); burada yeniden sıralanmaz. Seçim `App::selected_index`
/// ile gelir. Salt okunur çizim: kilit ya da iptal noktası yoktur.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let procs = app.visible_procs();
    let total = app.last.as_ref().map_or(0, |s| s.procs.len());
    let count = Line::from(Span::styled(
        format!(" {} / {} ", procs.len(), total),
        theme::fg(Tok::Muted),
    ));
    let block = widgets::panel_with("Süreçler", count).title_bottom(filter_line(app));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.last.is_none() {
        render_message(f, inner, "örnek bekleniyor…");
    } else if procs.is_empty() {
        render_message(f, inner, "okunabilen DRM istemcisi yok");
    } else {
        render_table(f, inner, area.width >= WIDE_MIN, app, &procs);
    }
}

/// Alt kenarlık başlığı: filtre aktifse (düzenleniyor ya da dolu) Accent, değilse Muted ipucu.
fn filter_line(app: &App) -> Line<'static> {
    if app.filter_editing {
        Line::from(Span::styled(
            format!(" / {}_ ", app.filter),
            theme::fg(Tok::Accent),
        ))
    } else if app.filter.is_empty() {
        Line::from(Span::styled(" / ile filtrele ", theme::fg(Tok::Muted)))
    } else {
        Line::from(Span::styled(
            format!(" / {} ", app.filter),
            theme::fg(Tok::Accent),
        ))
    }
}

/// Gövde alanının ortasına soluk tek satır bilgi iletisi yazar.
fn render_message(f: &mut Frame, area: Rect, msg: &str) {
    let line = widgets::centered(area, area.width, 1);
    f.render_widget(widgets::empty_state(msg), line);
}

/// Tabloyu çizer; başlıkta aktif sıralama vurgulu, taşma varsa sağda kaydırma çubuğu.
fn render_table(f: &mut Frame, area: Rect, wide: bool, app: &App, procs: &[&ProcUsage]) {
    let mark = sort_arrow(app.sort);
    let on = |k: SortKey| app.sort == k;
    let mut head = vec![
        header_cell("PID", on(SortKey::Pid), mark, true),
        header_cell("Ad", on(SortKey::Name), mark, false),
        // Busy sıralaması en yüksek motor yüzdesine göredir: GPU sütunu.
        header_cell("GPU", on(SortKey::Busy), mark, false),
    ];
    let mut widths = vec![
        Constraint::Length(7),
        Constraint::Min(12),
        Constraint::Length(GPU_W),
    ];
    if wide {
        for (name, _) in ENGINE_COLUMNS {
            head.push(header_cell(name, false, mark, true));
            widths.push(Constraint::Length(7));
        }
    }
    head.push(header_cell("VRAM", on(SortKey::Vram), mark, true));
    widths.push(Constraint::Length(9));
    let header = Row::new(head).style(theme::bold(Tok::Muted).bg(theme::color(Tok::Surface)));

    let rows: Vec<Row> = procs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let row = proc_row(p, wide);
            if i % 2 == 1 {
                row.style(Style::default().bg(theme::color(Tok::RowAlt)))
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(theme::selected());
    let mut state = TableState::default().with_selected(app.selected_index());
    f.render_stateful_widget(table, area, &mut state);

    let visible = usize::from(area.height.saturating_sub(1));
    if procs.len() > visible && visible > 0 {
        let mut sb = ScrollbarState::new(procs.len().saturating_sub(visible))
            .viewport_content_length(visible)
            .position(state.offset());
        let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("┃")
            .track_style(theme::fg(Tok::Surface))
            .thumb_style(theme::fg(Tok::Accent));
        // Başlık satırının altından başlar; sağ iç boşluk sütununa oturur.
        let sb_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width + 1,
            height: area.height - 1,
        };
        f.render_stateful_widget(bar, sb_area, &mut sb);
    }
}

/// Aktif sıralamanın yön oku: azalan ▼, artan ▲.
fn sort_arrow(sort: SortKey) -> &'static str {
    match sort {
        SortKey::Busy | SortKey::Vram => "▼",
        SortKey::Pid | SortKey::Name => "▲",
    }
}

/// Başlık hücresi: aktifse Accent + bold ve `mark` oku; değilse (satır stilinden) soluk.
fn header_cell(base: &str, active: bool, mark: &str, right: bool) -> Cell<'static> {
    let text = if active {
        format!("{base} {mark}")
    } else {
        base.to_owned()
    };
    let style = if active {
        theme::bold(Tok::Accent)
    } else {
        theme::bold(Tok::Muted)
    };
    aligned(Line::from(Span::styled(text, style)), right)
}

/// Hücre metnini isteğe bağlı sağa hizalar.
fn aligned(line: Line<'static>, right: bool) -> Cell<'static> {
    let text = Text::from(line);
    Cell::from(if right { text.right_aligned() } else { text })
}

/// Bir süreç satırı: PID, Ad, GPU çubuğu, (geniş düzende) motorlar ve VRAM.
fn proc_row(p: &ProcUsage, wide: bool) -> Row<'static> {
    let mut cells = vec![
        aligned(Line::from(p.pid.to_string()), true),
        Cell::from(p.comm.clone()),
        gpu_cell(p),
    ];
    if wide {
        cells.extend(
            ENGINE_COLUMNS
                .iter()
                .map(|&(_, class)| engine_cell(p, class)),
        );
    }
    cells.push(vram_cell(p));
    Row::new(cells)
}

/// Yüzde metni sağa hizalı `width` karakter; 0.0 → soluk `·`.
fn pct_span(pct: f32, width: usize) -> Span<'static> {
    if pct <= 0.0 {
        Span::styled(format!("{:>width$}", "·"), theme::fg(Tok::Muted))
    } else {
        Span::raw(format!("{:>width$}", fmt::pct(pct)))
    }
}

/// GPU sütunu: en yüksek motor yüzdesinin yük renkli çubuğu + yüzde.
fn gpu_cell(p: &ProcUsage) -> Cell<'static> {
    let max = p.engines.iter().map(|e| e.busy_pct).fold(0.0_f32, f32::max);
    let mut line = widgets::hbar(max, GPU_BAR, theme::load_color(max));
    line.spans.push(Span::raw(" "));
    line.spans.push(pct_span(max, 6));
    Cell::from(line)
}

/// Sürecin `class` motorundaki doluluk (sağa hizalı); yok ya da 0.0 ise soluk `·`.
fn engine_cell(p: &ProcUsage, class: EngineClass) -> Cell<'static> {
    let pct = p
        .engines
        .iter()
        .find(|e| e.class == class)
        .map_or(0.0, |e| e.busy_pct);
    aligned(Line::from(pct_span(pct, 7)), true)
}

/// Sürecin VRAM kullanımı (sağa hizalı); okunamıyorsa soluk `DASH`.
fn vram_cell(p: &ProcUsage) -> Cell<'static> {
    let span = match &p.vram_bytes {
        Metric::Value(b) => Span::raw(fmt::bytes(*b)),
        Metric::Unavailable(_) => Span::styled(fmt::DASH, theme::fg(Tok::Muted)),
    };
    aligned(Line::from(span), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use arctop_core::{Device, Driver};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::View;
    use crate::ui::draw;

    /// `last` değeri olmayan (henüz örnek gelmemiş) App üretir.
    fn empty_app() -> App {
        App::new(
            Device {
                card: 0,
                pci_slot: "0000:03:00.0".into(),
                device_id: 0x56a1,
                name: "Intel Arc A750".into(),
                driver: Driver::I915,
            },
            vec![],
            Duration::from_secs(1),
        )
    }

    /// Görünümü tek başına çizip düz metin olarak döndürür.
    fn view_text(width: u16, height: u16, app: &App) -> String {
        let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
        t.draw(|f| render(f, f.area(), app)).unwrap();
        t.backend().to_string()
    }

    /// Tüm ekranı (sekmeler + görünüm + alt satır) çizip düz metin döndürür.
    fn screen_text(width: u16, height: u16, app: &App) -> String {
        let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        t.backend().to_string()
    }

    #[test]
    fn snapshot_sample() {
        let app = App::sample();
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn snapshot_filtered() {
        let mut app = App::sample();
        app.filter = "e".into();
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        insta::assert_snapshot!(t.backend());
    }

    #[test]
    fn no_panic_at_80x24() {
        let mut app = App::sample();
        app.view = View::Procs;
        for sort in [SortKey::Busy, SortKey::Vram, SortKey::Pid, SortKey::Name] {
            app.sort = sort;
            let text = screen_text(80, 24, &app);
            assert!(text.contains("PID"), "başlık eksik ({sort:?})");
            assert!(text.contains("blender"), "satır eksik ({sort:?})");
            assert!(text.contains("5 / 5"), "sayı eksik ({sort:?})");
        }
    }

    #[test]
    fn no_panic_at_small_sizes() {
        // 40x10 ve 20x4: tablo, boş liste ve örneksiz durum panik atmamalı.
        let sample = App::sample();
        let waiting = empty_app();
        for (w, h) in [(40, 10), (20, 4)] {
            for sort in [SortKey::Busy, SortKey::Vram, SortKey::Pid, SortKey::Name] {
                let mut app = App::sample();
                app.sort = sort;
                view_text(w, h, &app);
            }
            view_text(w, h, &sample);
            view_text(w, h, &waiting);
        }
    }

    #[test]
    fn no_panic_and_message_when_no_snapshot() {
        let mut app = empty_app();
        app.view = View::Procs;
        let text = screen_text(80, 24, &app);
        assert!(text.contains("örnek bekleniyor"));
        assert!(text.contains("0 / 0"));
    }

    #[test]
    fn empty_list_message_and_zero_count() {
        let mut app = App::sample();
        app.filter = "zzz".into();
        let text = view_text(100, 30, &app);
        assert!(text.contains("okunabilen DRM istemcisi yok"));
        assert!(text.contains("0 / 5"));
    }

    #[test]
    fn filter_line_and_cursor() {
        let mut app = App::sample();
        app.filter = "fir".into();
        assert!(view_text(100, 30, &app).contains("/ fir"));
        app.filter_editing = true;
        assert!(view_text(100, 30, &app).contains("/ fir_"));
        let mut fresh = App::sample();
        // Boş filtre + düzenleme: yalnızca imleç görünür.
        fresh.filter_editing = true;
        assert!(view_text(100, 30, &fresh).contains("/ _"));
    }

    #[test]
    fn sort_marks_in_header() {
        let mut app = App::sample();
        for (sort, want) in [
            (SortKey::Busy, "GPU ▼"),
            (SortKey::Vram, "VRAM ▼"),
            (SortKey::Pid, "PID ▲"),
            (SortKey::Name, "Ad ▲"),
        ] {
            app.sort = sort;
            let text = view_text(100, 30, &app);
            assert!(text.contains(want), "{sort:?} başlığında yok: {want}");
        }
    }

    /// Başlık satırında (y=1) `needle`'ın başladığı sütun.
    fn header_col(t: &Terminal<TestBackend>, needle: &str) -> u16 {
        let b = t.backend().buffer();
        let row: Vec<&str> = (0..b.area.width).map(|x| b[(x, 1)].symbol()).collect();
        let hay: String = row.concat();
        let byte = hay.find(needle).expect("başlık yok");
        row[..]
            .iter()
            .scan(0, |acc, s| {
                let at = *acc;
                *acc += s.len();
                Some(at)
            })
            .position(|at| at == byte)
            .unwrap() as u16
    }

    #[test]
    fn sort_column_highlight_colors() {
        let mut app = App::sample();
        app.sort = SortKey::Busy;
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        let (gpu, vram) = (header_col(&t, "GPU"), header_col(&t, "VRAM"));
        let accent = theme::color(Tok::Accent);
        assert_eq!(t.backend().buffer()[(gpu, 1)].fg, accent);
        assert_ne!(t.backend().buffer()[(vram, 1)].fg, accent);
        app.sort = SortKey::Vram;
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        assert_eq!(t.backend().buffer()[(vram, 1)].fg, accent);
        assert_ne!(t.backend().buffer()[(gpu, 1)].fg, accent);
    }

    #[test]
    fn narrow_layout_hides_engine_columns() {
        let app = App::sample();
        let narrow = view_text(80, 24, &app);
        assert!(narrow.contains("GPU") && narrow.contains("VRAM"));
        assert!(!narrow.contains("Render") && !narrow.contains("Compute"));
        let wide = view_text(100, 30, &app);
        assert!(wide.contains("Render") && wide.contains("Compute"));
    }

    #[test]
    fn filter_hint_in_bottom_border() {
        let mut app = App::sample();
        assert!(view_text(100, 30, &app).contains("/ ile filtrele"));
        app.filter = "e".into();
        assert!(!view_text(100, 30, &app).contains("/ ile filtrele"));
    }

    /// Dar düzen ve filtreli/seçili/taşan görünümler için snapshot yardımcısı.
    fn draw_snapshot(w: u16, h: u16, app: &App) -> Terminal<TestBackend> {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| render(f, f.area(), app)).unwrap();
        t
    }

    #[test]
    fn snapshot_selected_third() {
        let mut app = App::sample();
        app.view = View::Procs;
        for _ in 0..2 {
            app.on_key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyModifiers::NONE,
            ));
        }
        assert_eq!(app.selected_index(), Some(2));
        insta::assert_snapshot!(draw_snapshot(100, 30, &app).backend());
    }

    #[test]
    fn snapshot_80x24() {
        insta::assert_snapshot!(draw_snapshot(80, 24, &App::sample()).backend());
    }

    #[test]
    fn snapshot_overflow_scrollbar() {
        let mut app = App::sample();
        if let Some(s) = app.last.as_mut() {
            let base = s.procs.clone();
            for i in 0..25u32 {
                let mut p = base[(i % 5) as usize].clone();
                p.pid = 1000 + i;
                s.procs.push(p);
            }
        }
        app.sort = SortKey::Pid;
        let t = draw_snapshot(100, 20, &app);
        let text = t.backend().to_string();
        assert!(text.contains('┃'), "kaydırma çubuğu yok");
        insta::assert_snapshot!(t.backend());
    }
}
