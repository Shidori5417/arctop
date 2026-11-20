//! Uygulama durumu ve tuş işleme (§6). Çizim yapmaz; yalnızca durumu değiştirir.

use std::time::{Duration, Instant};

use arctop_core::history::History;
use arctop_core::{Capabilities, Device, ProcUsage, Snapshot};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Örnekleme aralığı alt sınırı.
pub const MIN_INTERVAL: Duration = Duration::from_millis(250);
/// Örnekleme aralığı üst sınırı.
pub const MAX_INTERVAL: Duration = Duration::from_secs(5);
/// PgUp/PgDn ile seçimin kaydığı satır sayısı.
const PAGE: usize = 10;

/// Aktif görünüm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Özet görünümü (`1`).
    Overview,
    /// Grafikler (`2`).
    Charts,
    /// Süreç tablosu (`3`).
    Procs,
    /// Yardım (`F1`).
    Help,
}

/// Süreç tablosu sıralama anahtarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Motor doluluğu (azalan).
    Busy,
    /// VRAM (azalan).
    Vram,
    /// PID (artan).
    Pid,
    /// Ad (artan).
    Name,
}

impl SortKey {
    /// `s` ile döngüdeki sonraki anahtar.
    fn next(self) -> Self {
        match self {
            SortKey::Busy => SortKey::Vram,
            SortKey::Vram => SortKey::Pid,
            SortKey::Pid => SortKey::Name,
            SortKey::Name => SortKey::Busy,
        }
    }
}

/// Grafik zaman penceresi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartWindow {
    /// Son 60 saniye.
    Last60s,
    /// Son 5 dakika.
    Last5m,
}

/// Ana döngünün dış dünyada uygulaması gereken eylem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Örnekleyicinin aralığını değiştir (`sampler.set_interval`).
    SetInterval(Duration),
}

/// TUI durumu.
// Alanların bir kısmını T-010..T-013 görünümleri tüketir; o zamana dek okunmazlar.
#[allow(dead_code)]
pub struct App {
    /// Aktif görünüm.
    pub view: View,
    /// Süreç sıralaması.
    pub sort: SortKey,
    /// Grafik penceresi.
    pub window: ChartWindow,
    /// Süreç adı filtresi (büyük/küçük harf duyarsız alt dize).
    pub filter: String,
    /// `/` ile filtre düzenleniyor mu?
    pub filter_editing: bool,
    /// Gösterilen cihaz.
    pub device: Device,
    /// Kaynakların `probe` sonuçları.
    pub capabilities: Vec<Capabilities>,
    /// Son snapshot.
    pub last: Option<Snapshot>,
    /// Grafik geçmişi.
    pub history: History,
    /// Geçerli örnekleme aralığı.
    pub interval: Duration,
    /// Çıkış istendi mi?
    pub should_quit: bool,
    /// Son snapshot'ın geldiği an (canlılık noktası için).
    pub last_snapshot_at: Option<Instant>,
    /// F1 ile Yardım'dan dönülecek görünüm (Help olamaz).
    return_view: View,
    /// Seçili sürecin PID'i (seçim PID'e bağlıdır).
    sel_pid: Option<u32>,
    /// Seçimin son bilinen indeksi; PID kaybolunca en yakın indeksi bulmak için.
    sel_idx: usize,
}

impl App {
    /// Başlangıç durumu: Özet görünümü, Busy sıralaması, 60 sn pencere.
    pub fn new(device: Device, capabilities: Vec<Capabilities>, interval: Duration) -> Self {
        Self {
            view: View::Overview,
            sort: SortKey::Busy,
            window: ChartWindow::Last60s,
            filter: String::new(),
            filter_editing: false,
            device,
            capabilities,
            last: None,
            history: History::new(),
            interval: interval.clamp(MIN_INTERVAL, MAX_INTERVAL),
            should_quit: false,
            last_snapshot_at: None,
            return_view: View::Overview,
            sel_pid: None,
            sel_idx: 0,
        }
    }

    /// Yeni snapshot'ı geçmişe işler ve `last` olarak saklar.
    pub fn on_snapshot(&mut self, s: Snapshot) {
        self.history.push(&s);
        self.last = Some(s);
        self.last_snapshot_at = Some(Instant::now());
        self.reconcile_selection();
    }

    /// Yardım açıkken arkada çizilecek görünüm (Help açık değilse geçerli görünüm).
    pub fn help_return_view(&self) -> View {
        self.return_view
    }

    /// Son snapshot en fazla `interval / 2` önce geldiyse `true` (canlılık noktası yanar).
    pub fn is_fresh(&self, now: Instant) -> bool {
        self.last_snapshot_at
            .is_some_and(|t| t + self.interval / 2 > now)
    }

    /// Seçili sürecin PID'i (liste boşsa `None`).
    #[allow(dead_code)] // T-028 (Süreçler) kullanır.
    pub fn selected_pid(&self) -> Option<u32> {
        self.sel_pid
    }

    /// Seçili sürecin `visible_procs()` içindeki indeksi.
    pub fn selected_index(&self) -> Option<usize> {
        let pid = self.sel_pid?;
        self.visible_procs().iter().position(|p| p.pid == pid)
    }

    /// Seçimi görünür listeyle uzlaştırır: aynı PID varsa indeksini günceller; yoksa
    /// eski indekse en yakın satırı (sona taşarsa sonuncuyu) seçer; liste boşsa seçimi kaldırır.
    /// İlk kez liste dolunca ilk satır seçilir.
    fn reconcile_selection(&mut self) {
        let pids: Vec<u32> = self.visible_procs().iter().map(|p| p.pid).collect();
        if pids.is_empty() {
            self.sel_pid = None;
            self.sel_idx = 0;
            return;
        }
        let idx = self
            .sel_pid
            .and_then(|pid| pids.iter().position(|&p| p == pid))
            .unwrap_or_else(|| self.sel_idx.min(pids.len() - 1));
        self.sel_idx = idx;
        self.sel_pid = Some(pids[idx]);
    }

    /// Seçimi `f(mevcut, uzunluk)` ile verilen indekse taşır (sınırlara kırpılır).
    fn move_selection(&mut self, f: impl FnOnce(usize, usize) -> usize) {
        let pids: Vec<u32> = self.visible_procs().iter().map(|p| p.pid).collect();
        if pids.is_empty() {
            return;
        }
        let cur = self
            .sel_pid
            .and_then(|pid| pids.iter().position(|&p| p == pid))
            .unwrap_or(0);
        let idx = f(cur, pids.len()).min(pids.len() - 1);
        self.sel_idx = idx;
        self.sel_pid = Some(pids[idx]);
    }

    /// Görünümü değiştirir; Help'ten çıkılırsa dönüş görünümü de güncellenir.
    fn goto(&mut self, v: View) {
        self.view = v;
        self.return_view = v;
    }

    /// Tuşu işler; ana döngünün yapması gereken bir eylem varsa döndürür.
    pub fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return None;
        }
        if self.filter_editing {
            match key.code {
                KeyCode::Enter => self.filter_editing = false,
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filter_editing = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            }
            self.reconcile_selection();
            return None;
        }
        let action = self.handle_key(key);
        // Sıralama/filtre değişmiş olabilir: seçimi PID'e göre yeniden hizala.
        self.reconcile_selection();
        action
    }

    /// Filtre düzenlenmiyorken tuş işleme.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let in_procs = self.view == View::Procs;
        match key.code {
            KeyCode::Esc if self.view == View::Help => self.view = self.return_view,
            KeyCode::Up | KeyCode::Char('k') if in_procs => {
                self.move_selection(|c, _| c.saturating_sub(1));
            }
            KeyCode::Down | KeyCode::Char('j') if in_procs => {
                self.move_selection(|c, _| c + 1);
            }
            KeyCode::PageUp if in_procs => {
                self.move_selection(|c, _| c.saturating_sub(PAGE));
            }
            KeyCode::PageDown if in_procs => {
                self.move_selection(|c, _| c + PAGE);
            }
            KeyCode::Home if in_procs => self.move_selection(|_, _| 0),
            KeyCode::End if in_procs => self.move_selection(|_, n| n - 1),
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('1') => self.goto(View::Overview),
            KeyCode::Char('2') => self.goto(View::Charts),
            KeyCode::Char('3') => self.goto(View::Procs),
            KeyCode::Tab => {
                // Help'teyken Tab, Help'e girilmeden önceki görünümden devam eder.
                let next = match self.return_view {
                    View::Overview => View::Charts,
                    View::Charts => View::Procs,
                    _ => View::Overview,
                };
                self.goto(next);
            }
            KeyCode::F(1) => {
                if self.view == View::Help {
                    self.view = self.return_view;
                } else {
                    self.return_view = self.view;
                    self.view = View::Help;
                }
            }
            KeyCode::Char('s') => self.sort = self.sort.next(),
            KeyCode::Char('g') => {
                self.window = match self.window {
                    ChartWindow::Last60s => ChartWindow::Last5m,
                    ChartWindow::Last5m => ChartWindow::Last60s,
                };
            }
            KeyCode::Char('/') => self.filter_editing = true,
            KeyCode::Char('+') => return self.set_interval((self.interval * 2).min(MAX_INTERVAL)),
            KeyCode::Char('-') => return self.set_interval((self.interval / 2).max(MIN_INTERVAL)),
            _ => {}
        }
        None
    }

    /// Aralık gerçekten değiştiyse eylem döndürür (sınırda `None`).
    fn set_interval(&mut self, d: Duration) -> Option<Action> {
        if d == self.interval {
            return None;
        }
        self.interval = d;
        Some(Action::SetInterval(d))
    }

    /// Filtre + sıralama uygulanmış süreç listesi (procs görünümü kullanır).
    #[allow(dead_code)] // T-012 kullanır.
    pub fn visible_procs(&self) -> Vec<&ProcUsage> {
        let Some(snap) = &self.last else {
            return Vec::new();
        };
        let needle = self.filter.to_lowercase();
        let mut v: Vec<&ProcUsage> = snap
            .procs
            .iter()
            .filter(|p| needle.is_empty() || p.comm.to_lowercase().contains(&needle))
            .collect();
        let busy = |p: &ProcUsage| p.engines.iter().map(|e| e.busy_pct).fold(0.0_f32, f32::max);
        let vram = |p: &ProcUsage| p.vram_bytes.value().copied().unwrap_or(0);
        // Eşitlikte PID artan: sıralama kararlı ve deterministik.
        match self.sort {
            SortKey::Busy => v.sort_by(|a, b| busy(b).total_cmp(&busy(a)).then(a.pid.cmp(&b.pid))),
            SortKey::Vram => v.sort_by(|a, b| vram(b).cmp(&vram(a)).then(a.pid.cmp(&b.pid))),
            SortKey::Pid => v.sort_by_key(|p| p.pid),
            SortKey::Name => v.sort_by(|a, b| a.comm.cmp(&b.comm).then(a.pid.cmp(&b.pid))),
        }
        v
    }

    /// Testler için deterministik, dolu bir App.
    #[cfg(test)]
    pub fn sample() -> Self {
        use arctop_core::{Driver, EngineClass, EngineUsage, FreqInfo, Metric};
        use std::time::Instant;

        let device = Device {
            card: 1,
            pci_slot: "0000:03:00.0".into(),
            device_id: 0x56a1,
            name: "Intel Arc A750".into(),
            driver: Driver::I915,
        };
        let caps = Capabilities {
            fields: vec![
                ("freq.act", Metric::Value(())),
                ("fan_rpm", Metric::Unavailable("yok")),
            ],
        };
        let mut app = App::new(device, vec![caps], Duration::from_secs(1));
        let eng = |class, busy_pct| EngineUsage { class, busy_pct };
        let proc = |pid, comm: &str, busy, vram| ProcUsage {
            pid,
            comm: comm.into(),
            engines: vec![eng(EngineClass::Render, busy)],
            vram_bytes: vram,
        };
        let mib = 1024 * 1024;
        let procs = vec![
            proc(100, "firefox", 20.0, Metric::Value(300 * mib)),
            proc(200, "Xorg", 5.0, Metric::Value(100 * mib)),
            proc(300, "mpv", 60.0, Metric::Value(500 * mib)),
            proc(400, "blender", 90.0, Metric::Unavailable("yok")),
            proc(500, "steam", 20.0, Metric::Value(200 * mib)),
        ];
        for i in 0..300u32 {
            let f = i as f32;
            let mut s = Snapshot::empty(Instant::now());
            s.freq = FreqInfo {
                act: Metric::Value(300 + (i % 100) * 20),
                cur: Metric::Value(1200),
                min: Metric::Value(300),
                max: Metric::Value(2400),
                boost: Metric::Unavailable("yok"),
            };
            s.power_w = Metric::Value(10.0 + (f % 50.0));
            s.power_limit_w = Metric::Value(225.0);
            s.temp_c = Metric::Value(40.0 + (f % 30.0));
            s.fan_rpm = Metric::Unavailable("yok");
            s.voltage_mv = Metric::Value(900);
            s.engines = vec![
                eng(EngineClass::Render, f % 100.0),
                eng(EngineClass::Video, (f * 2.0) % 100.0),
            ];
            s.vram_used = Metric::Value(1024 * mib);
            s.vram_total = Metric::Value(8192 * mib);
            s.procs = procs.clone();
            app.on_snapshot(s);
        }
        app
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn ch(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    fn pids(app: &App) -> Vec<u32> {
        app.visible_procs().iter().map(|p| p.pid).collect()
    }

    #[test]
    fn view_switching_and_tab_cycle() {
        let mut app = App::sample();
        assert_eq!(app.view, View::Overview);
        app.on_key(ch('3'));
        assert_eq!(app.view, View::Procs);
        app.on_key(ch('2'));
        assert_eq!(app.view, View::Charts);
        app.on_key(ch('1'));
        for want in [View::Charts, View::Procs, View::Overview] {
            app.on_key(key(KeyCode::Tab));
            assert_eq!(app.view, want);
        }
    }

    #[test]
    fn f1_toggles_and_returns_to_previous_view() {
        let mut app = App::sample();
        app.on_key(ch('3'));
        app.on_key(key(KeyCode::F(1)));
        assert_eq!(app.view, View::Help);
        app.on_key(key(KeyCode::F(1)));
        assert_eq!(app.view, View::Procs);
        // Help'teyken Tab, dönüş görünümünün sonrasına gider (Help döngüde yok).
        app.on_key(key(KeyCode::F(1)));
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.view, View::Overview);
    }

    #[test]
    fn sort_cycles() {
        let mut app = App::sample();
        let mut seen = vec![app.sort];
        for _ in 0..4 {
            app.on_key(ch('s'));
            seen.push(app.sort);
        }
        assert_eq!(
            seen,
            [
                SortKey::Busy,
                SortKey::Vram,
                SortKey::Pid,
                SortKey::Name,
                SortKey::Busy
            ]
        );
    }

    #[test]
    fn filter_editing_accepts_q_and_esc_clears() {
        let mut app = App::sample();
        app.on_key(ch('/'));
        assert!(app.filter_editing);
        app.on_key(ch('q'));
        app.on_key(ch('s'));
        app.on_key(ch('x'));
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.filter, "qs");
        assert!(!app.should_quit);
        assert_eq!(
            app.sort,
            SortKey::Busy,
            "düzenlerken s sıralamayı değiştirmez"
        );
        app.on_key(key(KeyCode::Enter));
        assert!(!app.filter_editing);
        assert_eq!(app.filter, "qs");
        app.on_key(ch('/'));
        app.on_key(key(KeyCode::Esc));
        assert!(!app.filter_editing);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn quit_keys() {
        let mut app = App::sample();
        app.on_key(ch('q'));
        assert!(app.should_quit);
        let mut app = App::sample();
        app.on_key(ch('/'));
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit, "Ctrl-C filtre düzenlerken de çıkar");
    }

    #[test]
    fn interval_bounds_and_actions() {
        let mut app = App::sample();
        assert_eq!(app.interval, Duration::from_secs(1));
        assert_eq!(
            app.on_key(ch('-')),
            Some(Action::SetInterval(Duration::from_millis(500)))
        );
        assert_eq!(
            app.on_key(ch('-')),
            Some(Action::SetInterval(Duration::from_millis(250)))
        );
        assert_eq!(app.on_key(ch('-')), None);
        assert_eq!(app.interval, MIN_INTERVAL);
        for _ in 0..4 {
            assert!(app.on_key(ch('+')).is_some());
        }
        assert_eq!(app.interval, Duration::from_secs(4));
        assert_eq!(
            app.on_key(ch('+')),
            Some(Action::SetInterval(Duration::from_secs(5)))
        );
        assert_eq!(app.on_key(ch('+')), None);
        assert_eq!(app.interval, MAX_INTERVAL);
    }

    #[test]
    fn window_toggles() {
        let mut app = App::sample();
        app.on_key(ch('g'));
        assert_eq!(app.window, ChartWindow::Last5m);
        app.on_key(ch('g'));
        assert_eq!(app.window, ChartWindow::Last60s);
    }

    #[test]
    fn visible_procs_sorting() {
        let mut app = App::sample();
        assert_eq!(pids(&app), [400, 300, 100, 500, 200]); // busy azalan, eşitlikte pid
        app.sort = SortKey::Vram;
        assert_eq!(pids(&app), [300, 100, 500, 200, 400]); // Unavailable = 0
        app.sort = SortKey::Pid;
        assert_eq!(pids(&app), [100, 200, 300, 400, 500]);
        app.sort = SortKey::Name;
        assert_eq!(pids(&app), [200, 400, 100, 300, 500]); // Xorg, blender, firefox, mpv, steam
    }

    #[test]
    fn visible_procs_filter_case_insensitive() {
        let mut app = App::sample();
        app.filter = "XOR".into();
        assert_eq!(pids(&app), [200]);
        app.filter = "e".into(); // firefox, blender, steam
        assert_eq!(pids(&app), [400, 100, 500]);
        app.filter = "zzz".into();
        assert!(pids(&app).is_empty());
    }

    #[test]
    fn on_snapshot_updates_last_and_history() {
        let app = App::sample();
        assert!(app.last.is_some());
        assert_eq!(app.history.freq_act().len(), 300);
    }

    #[test]
    fn selection_moves_within_bounds_in_procs_only() {
        let mut app = App::sample();
        // Procs dışında hareket etmez ama ilk satır varsayılan seçili.
        assert_eq!(app.selected_pid(), Some(400));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.selected_pid(), Some(400));
        app.on_key(ch('3'));
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.selected_index(), Some(0), "üst sınır");
        app.on_key(key(KeyCode::Down));
        app.on_key(ch('j'));
        assert_eq!(app.selected_index(), Some(2));
        assert_eq!(app.selected_pid(), Some(100));
        app.on_key(ch('k'));
        assert_eq!(app.selected_index(), Some(1));
        app.on_key(key(KeyCode::End));
        assert_eq!(app.selected_index(), Some(4));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.selected_index(), Some(4), "alt sınır");
        app.on_key(key(KeyCode::PageUp));
        assert_eq!(app.selected_index(), Some(0));
        app.on_key(key(KeyCode::PageDown));
        assert_eq!(app.selected_index(), Some(4));
        app.on_key(key(KeyCode::Home));
        assert_eq!(app.selected_index(), Some(0));
    }

    #[test]
    fn selection_ignored_while_editing_filter() {
        let mut app = App::sample();
        app.on_key(ch('3'));
        app.on_key(ch('/'));
        app.on_key(ch('j'));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.filter, "j");
        assert_eq!(
            app.selected_pid(),
            None,
            "filtre 'j' hiçbir süreçle eşleşmez"
        );
    }

    #[test]
    fn selection_follows_pid_across_resort() {
        let mut app = App::sample();
        app.on_key(ch('3'));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.selected_pid(), Some(100));
        app.on_key(ch('s')); // Vram: 300, 100, 500, 200, 400
        assert_eq!(app.selected_pid(), Some(100));
        assert_eq!(app.selected_index(), Some(1));
        app.on_key(ch('s')); // Pid
        assert_eq!(app.selected_index(), Some(0));
    }

    #[test]
    fn selection_falls_back_to_nearest_index_when_pid_vanishes() {
        let mut app = App::sample();
        app.on_key(ch('3'));
        app.on_key(key(KeyCode::End)); // pid 200, indeks 4
        assert_eq!(app.selected_pid(), Some(200));
        let mut snap = app.last.clone().unwrap();
        snap.procs.retain(|p| p.pid != 200);
        app.on_snapshot(snap.clone());
        assert_eq!(
            app.selected_index(),
            Some(3),
            "sondan taşan en yakın indeks"
        );
        assert_eq!(app.selected_pid(), Some(500));
        snap.procs.retain(|p| p.pid != 300 && p.pid != 100);
        app.on_snapshot(snap.clone());
        assert_eq!(app.selected_pid(), Some(500));
        snap.procs.clear();
        app.on_snapshot(snap);
        assert_eq!(app.selected_pid(), None);
        assert_eq!(app.selected_index(), None);
    }

    #[test]
    fn esc_closes_help_and_is_noop_elsewhere() {
        let mut app = App::sample();
        app.on_key(ch('2'));
        app.on_key(key(KeyCode::F(1)));
        assert_eq!(app.view, View::Help);
        assert_eq!(app.help_return_view(), View::Charts);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.view, View::Charts);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.view, View::Charts);
    }

    #[test]
    fn is_fresh_uses_half_interval() {
        let mut app = App::sample(); // aralık 1 sn
        let t = app.last_snapshot_at.unwrap();
        assert!(app.is_fresh(t));
        assert!(app.is_fresh(t + Duration::from_millis(499)));
        assert!(!app.is_fresh(t + Duration::from_millis(500)));
        app.last_snapshot_at = None;
        assert!(!app.is_fresh(t));
    }
}
