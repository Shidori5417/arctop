//! Örnekleyici: metrik kaynaklarını periyodik olarak okuyup snapshot üretir (§5).
//!
//! # İş parçacığı modeli ve kilit sırası
//! Kilit (Mutex) kullanılmaz; yalnızca kanallar ve bir `AtomicBool` vardır.
//! * **Örnekleyici thread'i (S):** kaynakları sahiplenir; `select!` ile kontrol
//!   kanalını ve zamanlayıcıyı bekler. `Stop`/`SetInterval` bekleme sırasında anında etki eder.
//! * **İletici thread'i (F):** S'nin yazdığı iç kanalı (A, `bounded(2)`, taşınca en eski
//!   atılır) kullanıcıya verilen kanala (B, `bounded(2)`) aktarır. Neden var: crossbeam'de
//!   bir kanalın "son alıcısı düştü" bilgisi, gönderen tarafta tutulan Receiver klonu
//!   yüzünden görünmez olur. B'de yalnızca F gönderir, klon tutulmaz; B'nin Receiver'ı
//!   düşünce `send_timeout` `Disconnected` döner, F `Stop` gönderip çıkar.
//!
//! # İptal (cancellation)
//! `Stop` → S çıkar, A'nın göndericisi düşer → F kalan öğeleri en iyi çabayla aktarıp çıkar.
//! `stop()` önce S'yi join eder, sonra `done` bayrağını kaldırıp F'yi join eder;
//! yavaş tüketici F'yi en fazla ~20 ms oyalar. Receiver düşürülürse F `Stop` yollar.
//! Zamanlama: bir sonraki tetik = önceki tetik + aralık (sapma birikmez); bir aralıktan
//! fazla geride kalınırsa tetik "şimdi"ye çekilir (art arda patlama olmaz).

use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, SendTimeoutError, Sender, TrySendError, after, bounded, select};

use crate::model::{Capabilities, Device, Snapshot};
use crate::source::MetricSource;
use crate::sysfs::SysFs;

/// Örnekleyiciye gönderilen kontrol iletileri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Örnekleme aralığını değiştirir.
    SetInterval(Duration),
    /// Thread'i durdurur.
    Stop,
}

/// Arka plan örnekleyicisinin tutamacı.
pub struct Sampler {
    ctl: Sender<Control>,
    sampler: Option<JoinHandle<()>>,
    forwarder: Option<JoinHandle<()>>,
    done: Arc<AtomicBool>,
    caps: Vec<Capabilities>,
}

const MIN_INTERVAL: Duration = Duration::from_millis(1);

impl Sampler {
    /// Thread'i başlatır; her turda önce `Snapshot::empty(now)`, sonra tüm kaynakların `sample`'ı.
    /// İlk turdan önce her kaynağın `probe`'u çağıcı thread'de bir kez çağrılır.
    /// İlk örnek hemen alınır. Dönen Receiver düşürülürse thread'ler kapanır.
    pub fn spawn(
        fs: SysFs,
        dev: Device,
        mut sources: Vec<Box<dyn MetricSource>>,
        interval: Duration,
    ) -> (Sampler, Receiver<Snapshot>) {
        let caps: Vec<Capabilities> = sources.iter_mut().map(|s| s.probe(&fs, &dev)).collect();
        let (ctl_tx, ctl_rx) = bounded::<Control>(16);
        let (a_tx, a_rx) = bounded::<Snapshot>(2);
        let (b_tx, b_rx) = bounded::<Snapshot>(2);
        let done = Arc::new(AtomicBool::new(false));

        let a_rx_fwd = a_rx.clone();
        let sampler =
            thread::spawn(move || run_sampler(fs, dev, sources, interval, ctl_rx, a_tx, a_rx));
        let ctl_fwd = ctl_tx.clone();
        let done_fwd = Arc::clone(&done);
        let forwarder = thread::spawn(move || run_forwarder(a_rx_fwd, b_tx, ctl_fwd, done_fwd));

        (
            Sampler {
                ctl: ctl_tx,
                sampler: Some(sampler),
                forwarder: Some(forwarder),
                done,
                caps,
            },
            b_rx,
        )
    }

    /// Aralığı değiştirir; bekleyen tetik anında yeniden hesaplanır.
    pub fn set_interval(&self, d: Duration) {
        // Kanal doluysa/kapalıysa yok say: thread zaten kapanıyor ya da meşgul.
        let _ = self.ctl.try_send(Control::SetInterval(d));
    }

    /// `probe` sonuçları (kaynak sırasıyla).
    pub fn capabilities(&self) -> Vec<Capabilities> {
        self.caps.clone()
    }

    /// Thread'leri durdurup join eder.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.ctl.try_send(Control::Stop);
        if let Some(h) = self.sampler.take() {
            let _ = h.join();
        }
        self.done.store(true, Ordering::SeqCst);
        if let Some(h) = self.forwarder.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn take_sample(
    fs: &SysFs,
    dev: &Device,
    sources: &mut [Box<dyn MetricSource>],
    now: Instant,
) -> Snapshot {
    let mut snap = Snapshot::empty(now);
    for s in sources.iter_mut() {
        s.sample(fs, dev, now, &mut snap);
    }
    snap
}

/// Örnekleyici thread gövdesi (S).
fn run_sampler(
    fs: SysFs,
    dev: Device,
    mut sources: Vec<Box<dyn MetricSource>>,
    interval: Duration,
    ctl: Receiver<Control>,
    out: Sender<Snapshot>,
    out_rx: Receiver<Snapshot>,
) {
    let mut interval = interval.max(MIN_INTERVAL);
    let mut last = Instant::now();
    let mut next = last; // ilk örnek hemen
    loop {
        let wait = next.saturating_duration_since(Instant::now());
        select! {
            recv(ctl) -> msg => match msg {
                Ok(Control::SetInterval(d)) => {
                    interval = d.max(MIN_INTERVAL);
                    next = last + interval;
                }
                Ok(Control::Stop) | Err(_) => return,
            },
            recv(after(wait)) -> _ => {
                let snap = take_sample(&fs, &dev, &mut sources, Instant::now());
                if !send_drop_oldest(&out, &out_rx, snap) {
                    return;
                }
                last = next;
                next = last + interval;
                let now = Instant::now();
                if now.saturating_duration_since(next) > interval {
                    next = now;
                }
            }
        }
    }
}

/// Kanal doluysa en eski snapshot'ı atıp yeniden gönderir. Kanal kapandıysa `false`.
fn send_drop_oldest(tx: &Sender<Snapshot>, rx: &Receiver<Snapshot>, mut snap: Snapshot) -> bool {
    loop {
        match tx.try_send(snap) {
            Ok(()) => return true,
            Err(TrySendError::Full(s)) => {
                snap = s;
                let _ = rx.try_recv(); // en eskiyi at
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

/// İletici thread gövdesi (F): A → B. B'nin alıcısı düşerse `Stop` yollar.
fn run_forwarder(
    a: Receiver<Snapshot>,
    b: Sender<Snapshot>,
    ctl: Sender<Control>,
    done: Arc<AtomicBool>,
) {
    loop {
        let mut snap = match a.recv_timeout(Duration::from_millis(50)) {
            Ok(s) => s,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if done.load(Ordering::SeqCst) {
                    return;
                }
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
        };
        loop {
            match b.send_timeout(snap, Duration::from_millis(20)) {
                Ok(()) => break,
                Err(SendTimeoutError::Timeout(s)) => {
                    if done.load(Ordering::SeqCst) {
                        return;
                    }
                    snap = s;
                }
                Err(SendTimeoutError::Disconnected(_)) => {
                    let _ = ctl.try_send(Control::Stop);
                    return;
                }
            }
        }
    }
}

/// Ayrı thread olmadan tek seferlik iki örnek (Δ metrikler için); ikincisi döner.
/// Her kaynağın `sample`'ı iki kez çağrılır; arada `gap` kadar uyunur. `probe` çağrılmaz.
pub fn sample_once(
    fs: &SysFs,
    dev: &Device,
    sources: &mut [Box<dyn MetricSource>],
    gap: Duration,
) -> Snapshot {
    let _ = take_sample(fs, dev, sources, Instant::now());
    thread::sleep(gap);
    take_sample(fs, dev, sources, Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Driver;

    struct Nop;
    impl MetricSource for Nop {
        fn name(&self) -> &'static str {
            "nop"
        }
        fn probe(&mut self, _: &SysFs, _: &Device) -> Capabilities {
            Capabilities { fields: vec![] }
        }
        fn sample(&mut self, _: &SysFs, _: &Device, _: Instant, _: &mut Snapshot) {}
    }

    #[test]
    fn spawn_gets_snapshot_and_stops() {
        let dev = Device {
            card: 0,
            pci_slot: "0000:00:02.0".into(),
            device_id: 0,
            name: "t".into(),
            driver: Driver::Xe,
        };
        let (s, rx) = Sampler::spawn(
            SysFs::with_root("/nonexistent"),
            dev,
            vec![Box::new(Nop)],
            Duration::from_millis(10),
        );
        assert_eq!(s.capabilities().len(), 1);
        assert!(rx.recv_timeout(Duration::from_secs(2)).is_ok());
        s.stop();
    }
}
