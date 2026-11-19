//! `--once` çıktısı: script dostu JSON ve insan okunur metin özeti.

use std::cmp::Reverse;

use serde_json::{Map, Value};

use arctop_core::{Device, EngineUsage, FreqInfo, Metric, ProcUsage, Snapshot};

use crate::fmt;

/// Bir `Metric`'i JSON değerine çevirir.
///
/// `Value` ise dönüşür, `Unavailable` ise `null` döner ve nedenini
/// `unavailable` haritasına `key` altında yazar.
fn metric_field<T: Copy>(
    m: &Metric<T>,
    key: &str,
    conv: impl Fn(T) -> Value,
    unavailable: &mut Map<String, Value>,
) -> Value {
    match m {
        Metric::Value(v) => conv(*v),
        Metric::Unavailable(reason) => {
            unavailable.insert(key.to_string(), Value::from(*reason));
            Value::Null
        }
    }
}

/// f32 değeri 2 ondalığa yuvarlanmış f64'e çevirir; `0.7605677843093872` gibi
/// f32→f64 gürültüsü JSON'a taşınmaz.
fn round2(x: f32) -> f64 {
    (x as f64 * 100.0).round() / 100.0
}

/// Frekans alt nesnesi; okunamayan alanlar `freq_mhz.<alan>` anahtarıyla bildirilir.
fn freq_json(f: &FreqInfo, unavailable: &mut Map<String, Value>) -> Value {
    let mut m = Map::new();
    for (key, field) in [
        ("act", &f.act),
        ("cur", &f.cur),
        ("min", &f.min),
        ("max", &f.max),
        ("boost", &f.boost),
    ] {
        let un_key = format!("freq_mhz.{key}");
        m.insert(
            key.to_string(),
            metric_field(field, &un_key, Value::from, unavailable),
        );
    }
    Value::Object(m)
}

/// Motor kullanımını fdinfo anahtarlarına (`EngineClass::as_fdinfo_key`) göre haritaya çevirir.
fn engine_map(engines: &[EngineUsage]) -> Value {
    let mut m = Map::new();
    for e in engines {
        m.insert(
            e.class.as_fdinfo_key().to_string(),
            Value::from(round2(e.busy_pct)),
        );
    }
    Value::Object(m)
}

/// Süreç dizisi; sürecin VRAM'i okunamadıysa `procs[<pid>].vram_bytes` anahtarı kullanılır.
fn procs_json(procs: &[ProcUsage], unavailable: &mut Map<String, Value>) -> Value {
    let mut arr = Vec::with_capacity(procs.len());
    for p in procs {
        let mut m = Map::new();
        m.insert("pid".to_string(), Value::from(p.pid));
        m.insert("comm".to_string(), Value::from(p.comm.clone()));
        let key = format!("procs[{}].vram_bytes", p.pid);
        m.insert(
            "vram_bytes".to_string(),
            metric_field(&p.vram_bytes, &key, Value::from, unavailable),
        );
        m.insert("engines".to_string(), engine_map(&p.engines));
        arr.push(Value::Object(m));
    }
    Value::Array(arr)
}

/// `--once --json` çıktısı: düz ve script dostu JSON (derive edilmiş `Metric` biçimi kullanılmaz).
///
/// Okunamayan alanlar `null` olur; nedenleri `unavailable` haritasında, JSON yolunu
/// izleyen anahtarlarla yer alır (`vram_total_bytes`, `freq_mhz.boost`,
/// `procs[1815].vram_bytes` gibi). Motor anahtarları `EngineClass::as_fdinfo_key` değerleridir.
pub fn to_json(dev: &Device, s: &Snapshot) -> serde_json::Value {
    let mut unavailable = Map::new();
    let mut root = Map::new();
    root.insert("card".to_string(), Value::from(dev.card));
    root.insert("name".to_string(), Value::from(dev.name.clone()));
    root.insert("driver".to_string(), Value::from(dev.driver.to_string()));
    root.insert("pci_slot".to_string(), Value::from(dev.pci_slot.clone()));
    root.insert("freq_mhz".to_string(), freq_json(&s.freq, &mut unavailable));
    for (key, field) in [
        ("power_w", &s.power_w),
        ("power_limit_w", &s.power_limit_w),
        ("temp_c", &s.temp_c),
    ] {
        root.insert(
            key.to_string(),
            metric_field(field, key, |v| Value::from(round2(v)), &mut unavailable),
        );
    }
    for (key, field) in [("fan_rpm", &s.fan_rpm), ("voltage_mv", &s.voltage_mv)] {
        root.insert(
            key.to_string(),
            metric_field(field, key, Value::from, &mut unavailable),
        );
    }
    for (key, field) in [
        ("vram_used_bytes", &s.vram_used),
        ("vram_total_bytes", &s.vram_total),
    ] {
        root.insert(
            key.to_string(),
            metric_field(field, key, Value::from, &mut unavailable),
        );
    }
    root.insert("engines".to_string(), engine_map(&s.engines));
    root.insert("procs".to_string(), procs_json(&s.procs, &mut unavailable));
    root.insert("unavailable".to_string(), Value::Object(unavailable));
    Value::Object(root)
}

/// VRAM metriğini okunabilir boyuta çevirir; okunamıyorsa uzun çizgi yazar.
fn vram_text(m: &Metric<u64>) -> String {
    match m {
        Metric::Value(v) => fmt::bytes(*v),
        Metric::Unavailable(_) => fmt::DASH.to_string(),
    }
}

/// `--once` çıktısı: birkaç satırlık insan okunur özet (`fmt` biçimlendiricileriyle).
///
/// Süreçler VRAM kullanımına göre azalan sırada en fazla 3 tanesi listelenir.
pub fn to_text(dev: &Device, s: &Snapshot) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} — {} sürücüsü, card {}, {}",
        dev.name, dev.driver, dev.card, dev.pci_slot
    ));
    lines.push(format!(
        "Frekans: {} | Güç: {} / {} | Sıcaklık: {} | Fan: {} | Gerilim: {}",
        fmt::metric(&s.freq.act, "MHz"),
        fmt::metric(&s.power_w, "W"),
        fmt::metric(&s.power_limit_w, "W"),
        fmt::metric(&s.temp_c, "°C"),
        fmt::metric(&s.fan_rpm, "RPM"),
        fmt::metric(&s.voltage_mv, "mV"),
    ));
    let engines = if s.engines.is_empty() {
        fmt::DASH.to_string()
    } else {
        s.engines
            .iter()
            .map(|e| format!("{} {}", e.class.as_fdinfo_key(), fmt::pct(e.busy_pct)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    lines.push(format!("Motorlar: {engines}"));
    lines.push(format!(
        "VRAM: {} / {}",
        vram_text(&s.vram_used),
        vram_text(&s.vram_total)
    ));
    let mut top: Vec<&ProcUsage> = s.procs.iter().collect();
    top.sort_by_key(|p| Reverse(p.vram_bytes.value()));
    let procs = top
        .iter()
        .take(3)
        .map(|p| format!("{} ({})", p.comm, vram_text(&p.vram_bytes)))
        .collect::<Vec<_>>();
    let procs = if procs.is_empty() {
        fmt::DASH.to_string()
    } else {
        procs.join(", ")
    };
    lines.push(format!("Süreçler: {procs}"));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Instant;

    use arctop_core::{Driver, EngineClass};
    use serde_json::json;

    fn dev() -> Device {
        Device {
            card: 1,
            pci_slot: "0000:03:00.0".into(),
            device_id: 0x5691,
            name: "Intel Arc A750".into(),
            driver: Driver::I915,
        }
    }

    fn snap() -> Snapshot {
        Snapshot {
            at: Instant::now(),
            freq: FreqInfo {
                act: Metric::Value(600),
                cur: Metric::Value(600),
                min: Metric::Value(300),
                max: Metric::Value(2400),
                boost: Metric::Unavailable("sysfs'de yok"),
            },
            power_w: Metric::Value(35.2),
            power_limit_w: Metric::Value(190.0),
            temp_c: Metric::Value(48.0),
            fan_rpm: Metric::Value(720),
            voltage_mv: Metric::Value(755),
            engines: vec![
                EngineUsage {
                    class: EngineClass::Render,
                    busy_pct: 12.5,
                },
                EngineUsage {
                    class: EngineClass::Copy,
                    busy_pct: 0.0,
                },
            ],
            vram_used: Metric::Value(1157603328),
            vram_total: Metric::Unavailable("sysfs'de yok"),
            procs: vec![ProcUsage {
                pid: 1815,
                comm: "Hyprland".into(),
                engines: vec![EngineUsage {
                    class: EngineClass::Render,
                    busy_pct: 3.1,
                }],
                vram_bytes: Metric::Value(26214400),
            }],
        }
    }

    #[test]
    fn json_has_exact_keys() {
        let v = to_json(&dev(), &snap());
        let root = v.as_object().unwrap();
        let keys = [
            "card",
            "name",
            "driver",
            "pci_slot",
            "freq_mhz",
            "power_w",
            "power_limit_w",
            "temp_c",
            "fan_rpm",
            "voltage_mv",
            "vram_used_bytes",
            "vram_total_bytes",
            "engines",
            "procs",
            "unavailable",
        ];
        assert_eq!(root.len(), keys.len());
        for k in keys {
            assert!(root.contains_key(k), "eksik anahtar: {k}");
        }
        let freq = v["freq_mhz"].as_object().unwrap();
        assert_eq!(freq.len(), 5);
        for k in ["act", "cur", "min", "max", "boost"] {
            assert!(freq.contains_key(k), "eksik frekans anahtarı: {k}");
        }
        let proc = v["procs"][0].as_object().unwrap();
        assert_eq!(proc.len(), 4);
        for k in ["pid", "comm", "vram_bytes", "engines"] {
            assert!(proc.contains_key(k), "eksik süreç anahtarı: {k}");
        }
    }

    #[test]
    fn json_values_match_spec_example() {
        let v = to_json(&dev(), &snap());
        assert_eq!(v["card"], json!(1));
        assert_eq!(v["name"], json!("Intel Arc A750"));
        assert_eq!(v["driver"], json!("i915"));
        assert_eq!(v["pci_slot"], json!("0000:03:00.0"));
        assert_eq!(v["freq_mhz"]["act"], json!(600));
        assert_eq!(v["freq_mhz"]["cur"], json!(600));
        assert_eq!(v["freq_mhz"]["min"], json!(300));
        assert_eq!(v["freq_mhz"]["max"], json!(2400));
        assert_eq!(v["freq_mhz"]["boost"], json!(null));
        assert_eq!(v["power_w"].as_f64(), Some(35.2));
        assert_eq!(v["power_limit_w"].as_f64(), Some(190.0));
        assert_eq!(v["temp_c"].as_f64(), Some(48.0));
        assert_eq!(v["fan_rpm"], json!(720));
        assert_eq!(v["voltage_mv"], json!(755));
        assert_eq!(v["vram_used_bytes"], json!(1157603328));
        assert_eq!(v["vram_total_bytes"], json!(null));
        assert_eq!(v["engines"]["render"].as_f64(), Some(12.5));
        assert_eq!(v["engines"]["copy"].as_f64(), Some(0.0));
        assert_eq!(v["procs"][0]["pid"], json!(1815));
        assert_eq!(v["procs"][0]["comm"], json!("Hyprland"));
        assert_eq!(v["procs"][0]["vram_bytes"], json!(26214400));
        assert_eq!(v["procs"][0]["engines"]["render"].as_f64(), Some(3.1));
    }

    #[test]
    fn floats_are_rounded_to_two_decimals() {
        let mut s = snap();
        s.power_w = Metric::Value(0.7605678_f32);
        s.temp_c = Metric::Value(47.999_f32);
        s.engines = vec![EngineUsage {
            class: EngineClass::Render,
            busy_pct: 33.337_f32,
        }];
        let v = to_json(&dev(), &s);
        assert_eq!(v["power_w"].as_f64(), Some(0.76));
        assert_eq!(v["temp_c"].as_f64(), Some(48.0));
        assert_eq!(v["engines"]["render"].as_f64(), Some(33.34));
        let text = v.to_string();
        assert!(!text.contains("0.7605"), "uzun ondalık: {text}");
        assert!(!text.contains("33.337"), "uzun ondalık: {text}");
    }

    #[test]
    fn unavailable_map_holds_reasons() {
        let v = to_json(&dev(), &snap());
        let un = v["unavailable"].as_object().unwrap();
        assert_eq!(un.len(), 2);
        assert_eq!(un["vram_total_bytes"], json!("sysfs'de yok"));
        assert_eq!(un["freq_mhz.boost"], json!("sysfs'de yok"));
    }

    #[test]
    fn all_unavailable_snapshot_yields_nulls_and_map() {
        let v = to_json(&dev(), &Snapshot::empty(Instant::now()));
        for k in [
            "power_w",
            "power_limit_w",
            "temp_c",
            "fan_rpm",
            "voltage_mv",
            "vram_used_bytes",
            "vram_total_bytes",
        ] {
            assert!(v[k].is_null(), "{k} null olmalı");
        }
        for k in ["act", "cur", "min", "max", "boost"] {
            assert!(v["freq_mhz"][k].is_null(), "freq_mhz.{k} null olmalı");
        }
        assert_eq!(v["engines"].as_object().unwrap().len(), 0);
        assert_eq!(v["procs"].as_array().unwrap().len(), 0);
        let un = v["unavailable"].as_object().unwrap();
        assert_eq!(un.len(), 12); // 5 frekans + 7 üst düzey alan
        for k in ["freq_mhz.act", "power_w", "vram_total_bytes"] {
            assert_eq!(un[k], json!("örneklenmedi"), "unavailable[{k}]");
        }
    }

    #[test]
    fn proc_vram_unavailable_lands_in_map() {
        let mut s = snap();
        s.procs[0].vram_bytes = Metric::Unavailable("fdinfo'da yok");
        let v = to_json(&dev(), &s);
        assert_eq!(v["procs"][0]["vram_bytes"], json!(null));
        assert_eq!(
            v["unavailable"]["procs[1815].vram_bytes"],
            json!("fdinfo'da yok")
        );
    }

    #[test]
    fn text_summary_is_human_readable() {
        let t = to_text(&dev(), &snap());
        assert!(t.contains("Intel Arc A750"), "çıktı: {t}");
        assert!(t.contains("600 MHz"));
        assert!(t.contains("35.2 W"));
        assert!(t.contains("190 W"));
        assert!(t.contains("48 °C"));
        assert!(t.contains("720 RPM"));
        assert!(t.contains("755 mV"));
        assert!(t.contains("render 12.5%"));
        assert!(t.contains("copy 0.0%"));
        assert!(t.contains("VRAM: 1.1 GiB / —"));
        assert!(t.contains("Süreçler: Hyprland (25 MiB)"));
    }

    #[test]
    fn text_all_unavailable_shows_dashes() {
        let t = to_text(&dev(), &Snapshot::empty(Instant::now()));
        assert!(t.contains("Motorlar: —"), "çıktı: {t}");
        assert!(t.contains("VRAM: — / —"));
        assert!(t.contains("Süreçler: —"));
    }
}
