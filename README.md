# arctop

Intel Arc GPU'lar için terminal tabanlı canlı izleme aracı. Rust ve ratatui ile yazılır. nvtop'un Arc tarafında eksik kalan kısımlarını tamamlamak için tasarlandı.

- Frekans, güç (W), sıcaklık, fan, voltaj
- Motor doluluğu (render / copy / video / video-enhance / compute)
- Süreç bazlı GPU ve VRAM kullanımı (fdinfo)
- Canlı grafikler (60 sn ve 5 dk geçmiş)
- Root gerektirmez. Okunamayan metrikler "—" olarak gösterilir ve nedeni yardım ekranında açıklanır
- `--json` / `--once` ile scriptlerden kullanılabilir

**Birincil hedef:** Arc A750 (DG2, `i915` sürücüsü). **İkincil hedef:** `xe` sürücüsü ve Battlemage.
**Durum:** v0.1 — kullanılabilir

## Ekran görüntüsü

Özet görünümü (metin olarak; `--once` çıktısı değil, test snapshot'ından alınmıştır):

```text
┌ Özet ────────────────────────────────────────────────────────────────────────────────────────────┐
│Intel Arc A750 · i915 · 0000:03:00.0 · aralık: 1000 ms                                            │
│                                                                                                  │
│Frekans      ███████████████████████95% █████████████████████   2280 MHz · cur 1200 · 300–2400 MHz│
│Güç          ███████████████████               26%                                    59 W / 225 W│
│Sıcaklık     ██████████████████████████████████████69% █████████████                         69 °C│
│Fan                                                                                              —│
│Voltaj                                                                                      900 mV│
│VRAM         ████████                        13%                                 1.0 GiB / 8.0 GiB│
│                                                                                                  │
│Render       ██████████████████████████████████████99% ████████████████████████████████████  99.0%│
│Video        ██████████████████████████████████████98% ███████████████████████████████████   98.0%│
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Kurulum

```bash
cargo install --git https://github.com/Shidori5417/arctop arctop
```

Kaynaktan derleme:

```bash
git clone https://github.com/Shidori5417/arctop
cd arctop
cargo build --release
./target/release/arctop
```

AUR paketi (`packaging/aur/PKGBUILD`) yakında yayınlanacak.

**Gereksinimler:**
- Linux 6.8 veya üzeri
- `i915` veya `xe` sürücüsü ile çalışan bir Intel Arc kart
- Root gerekmez

## Kullanım

```bash
arctop                        # TUI
arctop --interval 500         # örnekleme aralığı (250–5000 ms, varsayılan 1000)
arctop --card 1               # birden fazla GPU varsa belirli bir kart (/sys/class/drm/card1)
arctop --once                 # tek ölçüm, metin çıktısı
arctop --once --json          # tek ölçüm, JSON
arctop --once --json | jq .power_w
arctop --record out.csv       # TUI açıkken her ölçümü CSV'ye ekle (--once ile birlikte kullanılamaz)
```

`--once` Δ gerektiren metrikler (güç, motor doluluğu) için iki örnek arasında `--interval` kadar bekler.

### Kısayollar

| Tuş | İşlev |
|---|---|
| `1-3` | görünüm (özet, grafikler, süreçler) |
| `Tab` | sonraki görünüm |
| `s` | sıralama sütunu |
| `/` | filtre |
| `+` / `-` | örnekleme aralığı |
| `g` | 60 sn / 5 dk grafik geçmişi |
| `c` | kart değiştir (birden fazla GPU varsa — henüz yok) |
| `F1` | yardım |
| `q` | çıkış |

### JSON çıktısı

`arctop --once --json` (gerçek çıktıdan kısaltılmış):

```json
{
  "card": 1,
  "driver": "i915",
  "name": "Intel Arc A750",
  "pci_slot": "0000:03:00.0",
  "freq_mhz": { "act": 0, "cur": 600, "min": 300, "max": 2400, "boost": 2400 },
  "power_w": 34.01,
  "power_limit_w": 190.0,
  "temp_c": 47.0,
  "fan_rpm": 119,
  "voltage_mv": 755,
  "engines": { "compute": 0.0, "copy": 0.0, "render": 0.46, "video": 0.0, "video-enhance": 0.0 },
  "vram_used_bytes": 1045979136,
  "vram_total_bytes": null,
  "unavailable": { "vram_total_bytes": "sysfs'de yok" },
  "procs": [
    { "pid": 1815, "comm": "Hyprland", "engines": { "render": 0.14, ... }, "vram_bytes": 244322304 },
    { "pid": 1921, "comm": "qs", "engines": { "render": 0.0, ... }, "vram_bytes": 28016640 }
    ...
  ]
}
```

## Neden nvtop değil?

nvtop genel amaçlı ve olgun bir araç; arctop onun yerine geçmez, Arc'ta eksik kalan kısımları tamamlar:

- **Arc'a özgü sensörler:** güç, fan ve voltaj hwmon üzerinden okunur.
- **Sürücüye göre doğru formüller:** `i915` ve `xe` için fdinfo yolları ayrı ayrı ele alınır.
- **Açıklayıcı eksik veri:** okunamayan her metrik "—" olarak gösterilir ve nedeni F1 ekranında yazılır.
- **Script dostu çıktı:** `--json` ve `--record` (CSV) ile otomasyona girer.

## Bilinen sınırlamalar

- i915'te VRAM toplamı sysfs'de yok; VRAM kullanımı gösterilir, toplam "—" olur.
- PMU henüz kullanılmıyor; motor doluluğu ve süreç verisi fdinfo'dan alınır. PMU için `perf_event_paranoid <= 0` veya `CAP_PERFMON` gerekir.
- `xe` sürücüsü yolları henüz gerçek donanımda doğrulanmadı (bkz. `docs/DECISIONS.md` ADR-007).
- Tek GPU desteklenir; TUI içinde kart değiştirme (`c`) henüz yok. Birden fazla kart için `--card` kullanılabilir.

## Geliştirme

Yol haritası: `docs/ROADMAP.md`. Belgeler: [SPEC](docs/SPEC.md), [ARCHITECTURE](docs/ARCHITECTURE.md), [SYSFS-NOTES](docs/SYSFS-NOTES.md), [DECISIONS](docs/DECISIONS.md).

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Donanım keşfi (sysfs ve fdinfo dökümü, geliştirici için):

```bash
scripts/discover-gpu.sh > docs/hw/a750.txt
```

Geliştirme, bir orkestratör ve alt ajanlarla (Claude, GLM, Antigravity) yürütülür. Ajan kuralları `CLAUDE.md` ve `AGENTS.md` dosyalarındadır.

## Lisans

MIT veya Apache-2.0, seçiminize bağlı. Bkz. [LICENSE-MIT](LICENSE-MIT) ve [LICENSE-APACHE](LICENSE-APACHE).
