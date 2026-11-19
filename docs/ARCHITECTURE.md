# arctop — Mimari

## Workspace düzeni
```
crates/
  arctop-core/   (lib)  keşif, metrik kaynakları, örnekleyici, geçmiş — UI bilmez
  arctop/        (bin)  CLI (clap), TUI (ratatui + crossterm), JSON/CSV çıktı
tests/fixtures/<makine>/   gerçek sysfs/proc ağaçlarının kopyası (Faz 0'da üretilir)
```
İki ayrı crate paralel çalışmayı kolaylaştırır: TUI ekipleri core'a dokunmaz.

## §1 SysFs soyutlaması (en kritik karar)
Tüm dosya okumaları `SysFs` üzerinden yapılır:
```rust
pub struct SysFs { root: PathBuf }   // gerçek sistemde "/", testte fixtures dizini
impl SysFs {
    pub fn read_u64(&self, p: impl AsRef<Path>) -> Result<u64, ReadError>;
    pub fn read_string(&self, p: impl AsRef<Path>) -> Result<String, ReadError>;
    pub fn read_dir(&self, p: impl AsRef<Path>) -> Result<Vec<PathBuf>, ReadError>;
    pub fn read_link_name(&self, p: impl AsRef<Path>) -> Result<String, ReadError>;
}
```
Fixture'larda sembolik bağ tutulmaz: bağın hedef adı `<bağ>.name` dosyasında saklanır (ör. `device/driver.name`). `read_link_name` gerçek bağ bulamazsa bu dosyaya düşer.

Bu sayede her kaynak, GPU'su olmayan bir makinede ya da CI'da fixture'larla test edilebilir. Doğrudan `std::fs` çağrısı **yasaktır** (PMU hariç).

## §2 Keşif (`core::discover`)
- `/sys/class/drm/card*` taranır, bağlayıcı girdileri (`card0-DP-1` gibi) atlanır
- `device/vendor == 0x8086` olanlar alınır; sürücü adı `device/driver` sembolik bağından okunur (`i915` | `xe`)
- PCI device id üzerinden okunabilir ad tablosu kullanılır (A750 = `0x56a1`, tablo `core::pci_ids` içinde)
- Çıktı: `Vec<Device { card: u32, pci_slot, device_id, name, driver: Driver }>`

## §3 Metrik kaynakları (`core::source`)
```rust
pub trait MetricSource: Send {
    fn name(&self) -> &'static str;
    /// Bu cihazda ne okunabildiğini bir kez belirler.
    fn probe(&mut self, fs: &SysFs, dev: &Device) -> Capabilities;
    /// Snapshot'taki kendi alanlarını doldurur. Δ hesabı için iç durum tutabilir.
    fn sample(&mut self, fs: &SysFs, dev: &Device, now: Instant, out: &mut Snapshot);
}
```
| Kaynak | Dosya | i915 / xe farkı |
|---|---|---|
| `freq` | `source/freq.rs` | i915: `cardN/gt_{act,cur,min,max,boost}_freq_mhz` · xe: `device/tile0/gt0/freq0/{act,cur,min,max}_freq` |
| `hwmon` | `source/hwmon.rs` | dosyalar `name` ve `*_label` ile eşlenir, sabit indeks varsayılmaz |
| `fdinfo` | `source/fdinfo.rs` | i915: `drm-engine-<sınıf>` ns · xe: `drm-cycles-<sınıf>` / `drm-total-cycles-<sınıf>` |
| `pmu` | `source/pmu.rs` (Faz 5) | `perf_event_open`; yalnızca burada `unsafe` kullanılır |

**fdinfo ayrıntıları:** Bir süreç aynı DRM istemcisini birden fazla fd ile tutabilir. Tekilleştirme `drm-client-id` ile yapılır. Doluluk = Δ(meşgul ns) / Δ(duvar saati ns), istemci başına hesaplanır. Önceki değerler `HashMap<(pdev, client_id), Counters>` içinde tutulur (ADR-011: paylaşılan istemci en küçük PID'e atanır), kaybolan süreçler temizlenir. Yalnızca hedef kartın `drm-pdev` değerine uyan istemciler sayılır.

## §4 Veri modeli (`core::model`)
```rust
pub enum Metric<T> { Value(T), Unavailable(&'static str) }
pub struct Snapshot {
    pub at: Instant,
    pub freq: FreqInfo,            // act, cur, min, max: Metric<u32>
    pub power_w: Metric<f32>, pub power_limit_w: Metric<f32>,
    pub temp_c: Metric<f32>, pub fan_rpm: Metric<u32>, pub voltage_mv: Metric<u32>,
    pub engines: Vec<EngineUsage>, // sınıf, %
    pub vram_used: Metric<u64>, pub vram_total: Metric<u64>,
    pub procs: Vec<ProcUsage>,     // pid, comm, motor %'leri, vram
}
```

## §5 Örnekleyici ve geçmiş (`core::sampler`, `core::history`)
- **tokio kullanılmaz.** Tek bir `std::thread` çalışır: `interval` aralıklarla tüm kaynaklar sırayla `sample` edilir ve `Snapshot` bir `crossbeam_channel` (bounded(2)) üzerinden UI'ya gönderilir. UI yavaşsa en eski snapshot atılır
- Aralık değişikliği ve durdurma ayrı bir kontrol kanalıyla iletilir
- `History`: metrik başına sabit kapasiteli halka tampon (300 nokta = 1 sn'de 5 dk)

## §6 TUI (`arctop::ui`)
- `App` durumu: aktif görünüm, sıralama, filtre, son snapshot, geçmiş
- Olay döngüsü: crossterm olayları ve snapshot kanalı `select!` ile dinlenir (crossbeam), çizim yalnızca değişiklik olduğunda yapılır
- Her görünüm kendi dosyasındadır: `ui/overview.rs`, `ui/charts.rs`, `ui/procs.rs`, `ui/help.rs`. İmza: `fn render(f: &mut Frame, area: Rect, app: &App)`
- Renkler `ui/theme.rs` içinde tanımlanır (Arc mavisi vurgu, terminal teması ile uyumlu)

## §7 Bağımlılıklar (sabit; yeni crate için ADR gerekir)
`ratatui`, `crossterm`, `clap` (derive), `crossbeam-channel`, `thiserror`, `anyhow`, `serde` + `serde_json`, `csv` (Faz 4), `libc` (Faz 5 PMU). Testlerde: `insta` (TUI snapshot testleri için `TestBackend`).

## §8 Test stratejisi
- Core: her kaynak `tests/fixtures/a750/` üzerinde test edilir. Δ gerektiren metrikler için iki ayrı fixture zamanı kullanılır (`t0/`, `t1/`)
- TUI: ratatui `TestBackend` + `insta` snapshot
- Gerçek donanım: `cargo run` ile her faz sonunda kullanıcı elle doğrular
