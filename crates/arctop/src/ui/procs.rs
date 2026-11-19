//! Süreç görünümü: GPU kullanan süreçler tablosu, filtre ve sıralama durumu (§6).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use arctop_core::{EngineClass, Metric, ProcUsage};

use crate::app::{App, SortKey};
use crate::fmt;
use crate::ui::theme;

/// Tablodaki motor sütunlarının sırası (Render..Compute grubu).
const ENGINE_COLUMNS: [EngineClass; 5] = [
    EngineClass::Render,
    EngineClass::Copy,
    EngineClass::Video,
    EngineClass::VideoEnhance,
    EngineClass::Compute,
];

/// Görünümü `area` içine çizer: süreç tablosu ve altta filtre/sayı satırı.
///
/// Satır sırası `App::visible_procs` tarafından belirlenir (filtre + sıralama
/// orada yapılmıştır); burada yeniden sıralama yapılmaz.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let [body, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let procs = app.visible_procs();
    let total = app.last.as_ref().map_or(0, |s| s.procs.len());
    if app.last.is_none() {
        render_message(f, body, "örnek bekleniyor…");
    } else if procs.is_empty() {
        render_message(f, body, "okunabilen DRM istemcisi yok");
    } else {
        render_table(f, body, app, &procs);
    }
    render_status(f, status, app, procs.len(), total);
}

/// Gövde alanına soluk tek satır bilgi iletisi yazar.
fn render_message(f: &mut Frame, area: Rect, msg: &str) {
    f.render_widget(Paragraph::new(msg).style(theme::dim()), area);
}

/// Tabloyu çizer; başlıkta aktif sıralama vurgulu, satırlar `procs` sırasıyla.
fn render_table(f: &mut Frame, area: Rect, app: &App, procs: &[&ProcUsage]) {
    let is_pid = app.sort == SortKey::Pid;
    let is_name = app.sort == SortKey::Name;
    let is_vram = app.sort == SortKey::Vram;
    let busy = app.sort == SortKey::Busy;
    let mark = sort_arrow(app.sort);
    // Busy sıralaması motor sınıflarının en yükseğine göredir: grup vurgulanır,
    // yön oku grubun son sütununda (Compute) gösterilir.
    let header = Row::new(vec![
        header_cell("PID", is_pid, if is_pid { mark } else { "" }),
        header_cell("Ad", is_name, if is_name { mark } else { "" }),
        header_cell("Render", busy, ""),
        header_cell("Copy", busy, ""),
        header_cell("Video", busy, ""),
        header_cell("VE", busy, ""),
        header_cell("Compute", busy, if busy { mark } else { "" }),
        header_cell("VRAM", is_vram, if is_vram { mark } else { "" }),
    ]);
    // 80 sütunlu terminalde Ad'a 24 karakter kalacak sabit genişlikler.
    let widths = [
        Constraint::Length(7), // PID
        Constraint::Min(4),    // Ad (kalan genişlik)
        Constraint::Length(7), // Render
        Constraint::Length(6), // Copy
        Constraint::Length(6), // Video
        Constraint::Length(6), // VE
        Constraint::Length(9), // Compute (Busy oku)
        Constraint::Length(8), // VRAM
    ];
    let rows = procs.iter().map(|p| proc_row(p));
    let table = Table::new(rows, widths).header(header).column_spacing(1);
    f.render_widget(table, area);
}

/// Aktif sıralamanın yön oku: azalan ▼, artan ▲.
fn sort_arrow(sort: SortKey) -> &'static str {
    match sort {
        SortKey::Busy | SortKey::Vram => "▼",
        SortKey::Pid | SortKey::Name => "▲",
    }
}

/// Başlık hücresi: vurguluysa Arc mavisi, değilse gri; `mark` boş değilse ok.
fn header_cell(base: &str, active: bool, mark: &str) -> Cell<'static> {
    let text = if mark.is_empty() {
        base.to_owned()
    } else {
        format!("{base} {mark}")
    };
    let style = if active { theme::title() } else { theme::tab() };
    Cell::from(Span::styled(text, style))
}

/// Bir süreç satırı: PID, Ad, motor dolulukları ve VRAM.
fn proc_row(p: &ProcUsage) -> Row<'static> {
    let mut cells = vec![Cell::from(p.pid.to_string()), Cell::from(p.comm.clone())];
    cells.extend(ENGINE_COLUMNS.iter().map(|&class| engine_cell(p, class)));
    cells.push(vram_cell(p));
    Row::new(cells)
}

/// Sürecin `class` motorundaki doluluk; o sınıf yoksa soluk `DASH`.
fn engine_cell(p: &ProcUsage, class: EngineClass) -> Cell<'static> {
    match p.engines.iter().find(|e| e.class == class) {
        Some(e) => Cell::from(fmt::pct(e.busy_pct)),
        None => Cell::from(Span::styled(fmt::DASH, theme::dim())),
    }
}

/// Sürecin VRAM kullanımı; okunamıyorsa soluk `DASH`.
fn vram_cell(p: &ProcUsage) -> Cell<'static> {
    match &p.vram_bytes {
        Metric::Value(b) => Cell::from(fmt::bytes(*b)),
        Metric::Unavailable(_) => Cell::from(Span::styled(fmt::DASH, theme::dim())),
    }
}

/// Alt satır: solda filtre durumu, sağda `görünen / toplam` süreç sayısı.
fn render_status(f: &mut Frame, area: Rect, app: &App, visible: usize, total: usize) {
    let count = format!("{visible} / {total}");
    let count_w = count.chars().count() as u16;
    let [filter_area, count_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(count_w)]).areas(area);
    f.render_widget(Paragraph::new(Line::from(filter_spans(app))), filter_area);
    f.render_widget(Paragraph::new(count), count_area);
}

/// Filtre satırı içerikleri: düzenleniyorsa imleçli filtre, boşsa soluk ipucu.
fn filter_spans(app: &App) -> Vec<Span<'static>> {
    if app.filter_editing {
        vec![
            Span::styled("/", theme::title()),
            Span::raw(format!(" {}_", app.filter)),
        ]
    } else if app.filter.is_empty() {
        vec![Span::styled("/ ile filtrele", theme::dim())]
    } else {
        vec![
            Span::styled("/", theme::title()),
            Span::raw(format!(" {}", app.filter)),
        ]
    }
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
            (SortKey::Busy, "Compute ▼"),
            (SortKey::Vram, "VRAM ▼"),
            (SortKey::Pid, "PID ▲"),
            (SortKey::Name, "Ad ▲"),
        ] {
            app.sort = sort;
            let text = view_text(100, 30, &app);
            assert!(text.contains(want), "{sort:?} başlığında yok: {want}");
        }
    }

    #[test]
    fn sort_column_highlight_colors() {
        let mut app = App::sample();
        app.sort = SortKey::Busy;
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        // 100 sütunda Render başlığı x=53'te başlar (7 + 1 + 44 + 1).
        assert_eq!(t.backend().buffer()[(53, 0)].fg, theme::ACCENT);
        assert_ne!(t.backend().buffer()[(92, 0)].fg, theme::ACCENT);
        app.sort = SortKey::Vram;
        t.draw(|f| render(f, f.area(), &app)).unwrap();
        assert_eq!(t.backend().buffer()[(92, 0)].fg, theme::ACCENT);
        assert_ne!(t.backend().buffer()[(53, 0)].fg, theme::ACCENT);
    }
}
