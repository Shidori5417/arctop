//! Gerçek sistemde uçtan uca örnek: keşif → kaynaklar → örnekleyici → 3 snapshot.
//!
//! Çalıştırma: `cargo run -p arctop-core --example dump`

use std::process;
use std::time::Duration;

use arctop_core::discover::discover;
use arctop_core::sampler::Sampler;
use arctop_core::source::fdinfo::FdinfoSource;
use arctop_core::source::freq::FreqSource;
use arctop_core::source::hwmon::HwmonSource;
use arctop_core::{MetricSource, SysFs};

/// Snapshot sayısı.
const SNAPSHOTS: usize = 3;
/// Yazdırılacak en fazla süreç sayısı.
const MAX_PROCS: usize = 5;

fn main() {
    let fs = SysFs::real();

    let devices = match discover(&fs) {
        Ok(devices) => devices,
        Err(e) => {
            eprintln!("keşif başarısız: {e}");
            process::exit(1);
        }
    };
    let Some(dev) = devices.into_iter().next() else {
        eprintln!("Intel GPU bulunamadı (/sys/class/drm altında kart yok)");
        process::exit(1);
    };
    println!("cihaz: card{} {} ({:?})", dev.card, dev.name, dev.driver);

    let sources: Vec<Box<dyn MetricSource>> = vec![
        Box::new(FreqSource::new()),
        Box::new(HwmonSource::new()),
        Box::new(FdinfoSource::new()),
    ];

    let (sampler, rx) = Sampler::spawn(fs, dev, sources, Duration::from_millis(1000));

    println!("--- capabilities ---");
    println!("{:#?}", sampler.capabilities());

    for i in 1..=SNAPSHOTS {
        let snap = match rx.recv() {
            Ok(snap) => snap,
            Err(e) => {
                eprintln!("örnekleyici kapandı: {e}");
                process::exit(1);
            }
        };
        println!("--- snapshot {i} (süreç toplam: {}) ---", snap.procs.len());
        let mut shown = snap;
        shown.procs.truncate(MAX_PROCS);
        println!("{shown:#?}");
    }

    sampler.stop();
}
