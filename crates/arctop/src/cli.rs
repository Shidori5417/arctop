//! Komut satırı parametreleri (SPEC "CLI").

/// arctop CLI parametreleri.
#[derive(clap::Parser, Debug)]
#[command(
    name = "arctop",
    version,
    about = "Intel Arc GPU'lar için terminal tabanlı izleme aracı"
)]
pub struct Cli {
    /// Örnekleme aralığı (ms), 250..=5000
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(250..=5000))]
    pub interval: u64,
    /// DRM kart numarası (/sys/class/drm/cardN). Verilmezse ilk Intel GPU.
    #[arg(long)]
    pub card: Option<u32>,
    /// Tek ölçüm alıp çık (Δ metrikler için iki örnek arasında --interval kadar bekler)
    #[arg(long)]
    pub once: bool,
    /// --once çıktısını JSON yaz
    #[arg(long, requires = "once")]
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn json_requires_once() {
        assert!(Cli::try_parse_from(["arctop", "--json"]).is_err());
    }

    #[test]
    fn interval_out_of_range_rejected() {
        assert!(Cli::try_parse_from(["arctop", "--interval", "100"]).is_err());
        assert!(Cli::try_parse_from(["arctop", "--interval", "6000"]).is_err());
    }

    #[test]
    fn interval_bounds_accepted() {
        assert_eq!(
            Cli::try_parse_from(["arctop", "--interval", "250"])
                .unwrap()
                .interval,
            250
        );
        assert_eq!(
            Cli::try_parse_from(["arctop", "--interval", "5000"])
                .unwrap()
                .interval,
            5000
        );
    }

    #[test]
    fn once_json_with_defaults() {
        let c = Cli::try_parse_from(["arctop", "--once", "--json"]).unwrap();
        assert!(c.once);
        assert!(c.json);
        assert_eq!(c.interval, 1000);
        assert_eq!(c.card, None);
    }

    #[test]
    fn card_and_interval_parsed() {
        let c = Cli::try_parse_from(["arctop", "--card", "2", "--interval", "500"]).unwrap();
        assert_eq!(c.card, Some(2));
        assert_eq!(c.interval, 500);
        assert!(!c.once);
        assert!(!c.json);
    }
}
