//! arctop: CLI ve TUI giriş noktası.

mod app;
mod fmt;
mod ui;

use std::io::{self, Stdout};
use std::panic;
use std::thread;
use std::time::Duration;

use anyhow::{Context, bail};
use arctop_core::discover::discover;
use arctop_core::sampler::Sampler;
use arctop_core::source::fdinfo::FdinfoSource;
use arctop_core::source::freq::FreqSource;
use arctop_core::source::hwmon::HwmonSource;
use arctop_core::{Device, MetricSource, SysFs};
use crossbeam_channel::{Receiver, select};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{Action, App};

/// Varsayılan örnekleme aralığı.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(1000);

fn main() -> anyhow::Result<()> {
    let fs = SysFs::real();
    let dev = first_device(&fs)?;
    run_tui(fs, dev, DEFAULT_INTERVAL)
}

/// İlk Intel GPU'yu bulur; yoksa anlaşılır hata döner.
fn first_device(fs: &SysFs) -> anyhow::Result<Device> {
    let devices = discover(fs).context("GPU keşfi başarısız")?;
    match devices.into_iter().next() {
        Some(d) => Ok(d),
        None => bail!("Intel GPU bulunamadı (/sys/class/drm altında i915/xe kartı yok)"),
    }
}

/// Varsayılan metrik kaynakları.
fn default_sources() -> Vec<Box<dyn MetricSource>> {
    vec![
        Box::new(FreqSource::new()),
        Box::new(HwmonSource::new()),
        Box::new(FdinfoSource::new()),
    ]
}

/// Örnekleyiciyi başlatıp TUI'yı çalıştırır; terminal her çıkış yolunda geri yüklenir.
fn run_tui(fs: SysFs, dev: Device, interval: Duration) -> anyhow::Result<()> {
    let (sampler, snap_rx) = Sampler::spawn(fs, dev.clone(), default_sources(), interval);
    let mut app = App::new(dev, sampler.capabilities(), interval);

    install_panic_hook();
    let mut terminal = init_terminal()?;
    // Guard, `?` ile dönüşte ve unwind'de Drop ile terminali geri yükler.
    let _guard = TerminalGuard;
    let result = event_loop(&mut terminal, &mut app, &sampler, &snap_rx);
    sampler.stop();
    result
}

/// Olay döngüsü: crossterm olayları ve snapshot kanalı `select!` ile dinlenir.
///
/// İptal/kapanış: `app.should_quit` ya da bir kanalın kapanması döngüyü bitirir.
/// Olay thread'i `event::read` içinde bloklu kalabilir; ana thread dönünce süreç
/// çıkarken sonlanır (detached). Çizim yalnızca bir olay/snapshot işlendiğinde yapılır.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    sampler: &Sampler,
    snap_rx: &Receiver<arctop_core::Snapshot>,
) -> anyhow::Result<()> {
    let ev_rx = spawn_event_reader();
    terminal.draw(|f| ui::draw(f, app))?;
    while !app.should_quit {
        select! {
            recv(ev_rx) -> ev => match ev {
                Ok(Ok(Event::Key(k))) => {
                    if let Some(Action::SetInterval(d)) = app.on_key(k) {
                        sampler.set_interval(d);
                    }
                }
                Ok(Ok(Event::Resize(..))) => {}
                Ok(Ok(_)) => continue, // fare/odak olayları: yeniden çizme
                Ok(Err(e)) => return Err(e).context("terminal olayı okunamadı"),
                Err(_) => bail!("olay okuyucu thread kapandı"),
            },
            recv(snap_rx) -> snap => match snap {
                Ok(s) => app.on_snapshot(s),
                Err(_) => bail!("örnekleyici beklenmedik şekilde kapandı"),
            },
        }
        terminal.draw(|f| ui::draw(f, app))?;
    }
    Ok(())
}

/// crossterm olaylarını ayrı thread'de okuyup kanala iletir.
/// Alıcı düşünce `send` hata verir ve thread biter.
fn spawn_event_reader() -> Receiver<io::Result<Event>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        loop {
            let ev = event::read();
            let failed = ev.is_err();
            if tx.send(ev).is_err() || failed {
                break;
            }
        }
    });
    rx
}

/// Raw mode + alternate screen açar.
fn init_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("raw mode açılamadı")?;
    let mut out = io::stdout();
    if let Err(e) = execute!(out, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(e).context("alternate screen açılamadı");
    }
    Ok(Terminal::new(CrosstermBackend::new(out))?)
}

/// Terminali geri yükler; birden çok kez çağrılması zararsızdır.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Drop'ta terminali geri yükler.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Panikte terminali geri yükleyip önceki hook'u çağırır (zincirleme).
fn install_panic_hook() {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        prev(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sources_has_three() {
        assert_eq!(default_sources().len(), 3);
    }
}
