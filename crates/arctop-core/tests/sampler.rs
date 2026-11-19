//! Örnekleyici entegrasyon testleri (sahte kaynakla; zamanlama toleransları geniş).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use arctop_core::sampler::{Sampler, sample_once};
use arctop_core::{Capabilities, Device, Driver, Metric, MetricSource, Snapshot, SysFs};

struct Fake {
    count: Arc<AtomicUsize>,
    probes: Arc<AtomicUsize>,
    _token: Arc<()>,
}

impl MetricSource for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn probe(&mut self, _: &SysFs, _: &Device) -> Capabilities {
        self.probes.fetch_add(1, Ordering::SeqCst);
        Capabilities {
            fields: vec![("power_w", Metric::Value(()))],
        }
    }
    fn sample(&mut self, _: &SysFs, _: &Device, _: Instant, out: &mut Snapshot) {
        let n = self.count.fetch_add(1, Ordering::SeqCst);
        out.power_w = Metric::Value(n as f32);
    }
}

fn dev() -> Device {
    Device {
        card: 0,
        pci_slot: "0000:00:02.0".into(),
        device_id: 0,
        name: "t".into(),
        driver: Driver::I915,
    }
}

struct Rig {
    sampler: Sampler,
    rx: crossbeam_channel::Receiver<Snapshot>,
    count: Arc<AtomicUsize>,
    probes: Arc<AtomicUsize>,
    token: Arc<()>,
}

fn rig(interval: Duration) -> Rig {
    let count = Arc::new(AtomicUsize::new(0));
    let probes = Arc::new(AtomicUsize::new(0));
    let token = Arc::new(());
    let src = Fake {
        count: count.clone(),
        probes: probes.clone(),
        _token: token.clone(),
    };
    let (sampler, rx) = Sampler::spawn(
        SysFs::with_root("/nonexistent"),
        dev(),
        vec![Box::new(src)],
        interval,
    );
    Rig {
        sampler,
        rx,
        count,
        probes,
        token,
    }
}

fn count_for(r: &Rig, d: Duration) -> usize {
    let end = Instant::now() + d;
    let mut n = 0;
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        if r.rx.recv_timeout(left).is_ok() {
            n += 1;
        }
    }
    n
}

#[test]
fn rate_changes_and_stop_is_fast() {
    let r = rig(Duration::from_millis(20));
    assert_eq!(r.probes.load(Ordering::SeqCst), 1);
    assert_eq!(r.sampler.capabilities().len(), 1);
    let n = count_for(&r, Duration::from_millis(110));
    assert!((3..=8).contains(&n), "20ms: {n}");
    r.sampler.set_interval(Duration::from_millis(100));
    std::thread::sleep(Duration::from_millis(40));
    while r.rx.try_recv().is_ok() {}
    let n2 = count_for(&r, Duration::from_millis(450));
    assert!((2..=7).contains(&n2), "100ms: {n2}");
    let t = Instant::now();
    r.sampler.stop();
    assert!(
        t.elapsed() < Duration::from_millis(100),
        "stop {:?}",
        t.elapsed()
    );
}

#[test]
fn slow_consumer_drops_oldest_keeps_newest() {
    let r = rig(Duration::from_millis(5));
    let mut ats = Vec::new();
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(60));
        if let Ok(s) = r.rx.recv_timeout(Duration::from_secs(1)) {
            ats.push(s.at);
        }
    }
    // Tüketici hızlanınca en yeni snapshot'a ulaşmalı.
    std::thread::sleep(Duration::from_millis(100));
    let mut last = None;
    while let Ok(s) = r.rx.try_recv() {
        ats.push(s.at);
        last = Some(s.at);
    }
    assert!(ats.windows(2).all(|w| w[0] < w[1]), "artan değil");
    // Çok sayıda örnek alındı ama tüketici az gördü: atılma oldu.
    assert!(r.count.load(Ordering::SeqCst) > ats.len() + 5);
    let last = last.expect("kuyrukta snapshot olmalı");
    assert!(
        last.elapsed() < Duration::from_millis(200),
        "en yeni değil: {:?}",
        last.elapsed()
    );
    r.sampler.stop();
}

#[test]
fn dropping_receiver_shuts_threads_down() {
    let r = rig(Duration::from_millis(10));
    assert!(r.rx.recv_timeout(Duration::from_secs(1)).is_ok());
    let Rig {
        sampler, rx, token, ..
    } = r;
    drop(rx);
    // Kaynak thread'e taşındı; thread çıkınca token serbest kalır.
    let end = Instant::now() + Duration::from_secs(3);
    while Arc::strong_count(&token) > 1 && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(Arc::strong_count(&token), 1, "thread kapanmadı");
    let t = Instant::now();
    sampler.stop();
    assert!(t.elapsed() < Duration::from_millis(500));
}

#[test]
fn sample_once_calls_sample_twice() {
    let count = Arc::new(AtomicUsize::new(0));
    let src = Fake {
        count: count.clone(),
        probes: Arc::new(AtomicUsize::new(0)),
        _token: Arc::new(()),
    };
    let mut v: Vec<Box<dyn MetricSource>> = vec![Box::new(src)];
    let s = sample_once(
        &SysFs::with_root("/nonexistent"),
        &dev(),
        &mut v,
        Duration::from_millis(5),
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert_eq!(s.power_w, Metric::Value(1.0));
}
