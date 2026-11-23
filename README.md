# arctop

[![CI](https://github.com/Shidori5417/arctop/actions/workflows/ci.yml/badge.svg)](https://github.com/Shidori5417/arctop/actions/workflows/ci.yml)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#lisans)

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

Ekran görüntüleri (metin olarak; test snapshot'larından alınmıştır, `--once` çıktısı değildir):

**Özet**

```text
 ◆ arctop   1 Özet  2 Grafikler  3 Süreçler                          Intel Arc A750 · i915  ● 1.0 s
╭ Frekans ─── maks 2400 ╮╭ Güç ───── limit 225 W ╮╭ Sıcaklık ─────────────╮╭ VRAM ─────────────────╮
│ 2.3 GHz               ││ 59.0 W                ││ 69 °C                 ││ 1.0 GiB               │
│ cur 1200 · 300–2400   ││ %26 limit             ││ 900 mV                ││ %12 / 8.0 GiB         │
│ ▄▄▄▄▅▅▅▅▅▅▅▆▆▆▆▆▆▆▆▇▇ ││                       ││         ▁▁▁▁▁▁▂▂▂▂▂▂▃ ││                       │
│ █████████████████████ ││ ▂▂▂▂▃▃▃▃▃▃▃▃▃▃▃▃▃▃▄▄▄ ││ ▇████████████████████ ││                       │
╰───────────────────────╯╰───────────────────────╯╰───────────────────────╯╰───────────────────────╯
╭ Motorlar ───────────────────────────── en yoğun %99 ╮╭ En çok kullananlar ───────────────────────╮
│ Render       ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  99.0% ││ blender        90% ━━━━━━━━━━─         —  │
│ Copy         ───────────────────────────────   0.0% ││ mpv            60% ━━━━━━━────   500 MiB  │
│ Video        ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━─  98.0% ││ firefox        20% ━━─────────   300 MiB  │
│ Video-Enh.   ───────────────────────────────   0.0% ││ steam          20% ━━─────────   200 MiB  │
│ Compute      ───────────────────────────────   0.0% ││ Xorg            5% ━──────────   100 MiB  │
╰─────────────────────────────────────────────────────╯╰───────────────────────────────────────────╯
╭ Etkinlik ────────────────────────────────────────────────────────── 60 sn · en yoğun motor / güç ╮
│ 100%│        ⢀⣀⠤⠔⠒⢹                                                               ⢀⣀⣀⣀⣀⠤⠤⠤⠒⠒⠒⠉⠉⠉ │
│     │ ⣀⡠⠤⠤⠔⠒⠉⠁     ⡇                                                ⢀⣀⣀⡠⠤⠤⠤⠤⠔⠒⠒⠊⠉⠉⠁              │
│     │⠉             ⢇                                   ⣀⣀⣀⠤⠤⠤⠒⠒⠒⠒⠊⠉⠉⠁                            │
│     │              ⢸                   ⢀⣀⣀⣀⣀⠤⠤⠤⠒⠒⠒⠊⠉⠉⠉⠉                                          │
│     │              ⠘⡄    ⢀⣀⣀⣀⠤⠤⠤⠤⠔⠒⠒⠊⠉⠉⠁                                                         │
│ 50% │               ⠓⠒⠊⠉⠉⠁                                                                       │
│     │                                                                                            │
│     │           ⢀⣀⣀                                                                          ⣀⣀⣀ │
│     │⠒⠒⠒⠒⠉⠉⠉⠉⠉⠉⠉⠁ ⠈⡆                                         ⣀⣀⣀⣀⣀⣀⣀⡠⠤⠤⠤⠤⠤⠤⠤⠤⠤⠒⠒⠒⠒⠒⠒⠒⠊⠉⠉⠉⠉⠉⠉⠉    │
│     │              ⢱         ⣀⣀⣀⣀⣀⣀⣀⡠⠤⠤⠤⠤⠤⠤⠤⠤⠔⠒⠒⠒⠒⠒⠒⠒⠊⠉⠉⠉⠉⠉⠉⠉                                    │
│ 0%  │               ⠓⠉⠉⠉⠉⠉⠉⠉⠉                                                                    │
│     └─────────────────────────────────────────────────────────────────────────────────────────── │
│                                                                                            şimdi │
╰───────────────────────────────────────── ━ motor  ━ güç ─────────────────────────────────────────╯
 1-3 görünüm  +/- aralık  F1 yardım  q çıkış
```

**Süreçler**

```text
╭ Süreçler ───────────────────────────────────────────────────────────────────────────────── 5 / 5 ╮
│     PID Ad                   GPU ▼              Render    Copy   Video      VE Compute      VRAM │
│     400 blender              ━━━━━━━━━─  90.0%   90.0%       ·       ·       ·       ·         — │
│     300 mpv                  ━━━━━━────  60.0%   60.0%       ·       ·       ·       ·   500 MiB │
│     100 firefox              ━━────────  20.0%   20.0%       ·       ·       ·       ·   300 MiB │
│     500 steam                ━━────────  20.0%   20.0%       ·       ·       ·       ·   200 MiB │
│     200 Xorg                 ━─────────   5.0%    5.0%       ·       ·       ·       ·   100 MiB │
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
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
╰ / ile filtrele ──────────────────────────────────────────────────────────────────────────────────╯
```

**Grafikler**

```text
Pencere:  60 sn   5 dk
╭ Frekans ────────────────────────────── 2.3 GHz ╮╭ Güç ─────────────────────────────────── 59.0 W ╮
│ 2.3 GHz│                               ⣀⣀⠤⠒⠒⠉⠉ ││ 59.0 W│  ⢀⠤⠔⢹                             ⡠⠤⠊⠉ │
│        │                        ⣀⣀⠤⠔⠒⠉⠉        ││       │⠔⠊⠁  ⢸                         ⢀⡠⠒⠊     │
│        │                 ⢀⣀⠤⠔⠒⠊⠉               ││       │     ⠈⡆                     ⡠⠤⠊⠁        │
│        │          ⢀⣀⡠⠔⠒⠊⠉⠁                     ││       │      ⡇                 ⢀⠤⠒⠉            │
│        │   ⢀⣀⡠⠤⠔⠊⠉⠁                            ││       │      ⡇             ⢀⡠⠔⠉⠁               │
│ 1.1 GHz│⠔⠒⠊⠁                                   ││ 29.5 W│      ⡇          ⢀⠔⠒⠁                   │
│        │                                       ││       │      ⢇      ⢀⠤⠔⠉⠁                      │
│        │                                       ││       │      ⢸   ⣀⠔⠒⠁                          │
│        │                                       ││       │      ⢸⠤⠒⠉                              │
│        │                                       ││       │                                        │
│ 0 MHz  │                                       ││ 0.0 W │                                        │
│        └────────────────────────────────────── ││       └─────────────────────────────────────── │
│   -60 sn               -30 sn            şimdi ││  -60 sn                 -30 sn           şimdi │
╰───────── 1.1 GHz · 1.7 GHz · 2.3 GHz ──────────╯╰─────────── 10.0 W · 37.8 W · 59.0 W ───────────╯
╭ Sıcaklık ─────────────────────────────── 69 °C ╮╭ En yoğun motor ────────────────────────── 99 % ╮
│ 100 °C│                                        ││ 100 %│   ⣀⡠⠒⡇                          ⣀⣀⠤⠔⠒⠊⠉ │
│       │                                        ││      │⣀⠤⠊   ⡇                   ⢀⣀⡠⠤⠒⠒⠉        │
│       │                                        ││      │      ⢣             ⣀⠤⠤⠒⠊⠉⠁              │
│       │            ⣀⣀⠤⠤⠒⠊⠹⡀           ⢀⣀⡠⠤⠔⠒⠉⠉ ││      │      ⢸     ⢀⣀⡠⠤⠔⠒⠉⠉                     │
│       │     ⢀⣀⡠⠤⠒⠒⠉       ⡇     ⣀⣀⠤⠔⠒⠊⠁        ││      │      ⢸⣀⠤⠒⠒⠉⠁                            │
│ 50 °C │⣀⠤⠔⠒⠊⠁             ⢱⡠⠤⠒⠒⠉               ││ 50 % │                                         │
│       │                                        ││      │                                         │
│       │                                        ││      │                                         │
│       │                                        ││      │                                         │
│ 0 °C  │                                        ││ 0 %  │                                         │
│       └─────────────────────────────────────── ││      └──────────────────────────────────────── │
│  -60 sn                 -30 sn           şimdi ││ -60 sn                 -30 sn            şimdi │
╰──────────── 40 °C · 54 °C · 69 °C ─────────────╯╰────────────── 50 % · 77 % · 99 % ──────────────╯
```

Renkler: truecolor, 256 renk ve `NO_COLOR` desteklenir.

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

`--record` dosyası yoksa oluşturur, varsa sonuna ekler. İlk sütun `session_start` (RFC 3339, UTC) her kayıt oturumunda sabittir; `elapsed_ms` ise her oturumda 0'dan başladığı için aynı dosyaya eklenen oturumlar bu sütunla ayırt edilir. `session_start` sütunu olmayan eski bir arctop dosyasına eklenirse biçimler karışmasın diye satırlar eski 14 sütunlu biçimde yazılır ve bir uyarı basılır; arctop kaydı olmayan (başlığı tanınmayan) dosya ise reddedilir, yeni bir dosya adı kullanın.

`--once` Δ gerektiren metrikler (güç, motor doluluğu) için iki örnek arasında `--interval` kadar bekler.

### Kısayollar

| Tuş | İşlev |
|---|---|
| `1-3` | görünüm (özet, grafikler, süreçler) |
| `Tab` | sonraki görünüm |
| `↑↓` / `j k` | seçim |
| `PgUp/PgDn` | sayfa |
| `Home/End` | başa / sona |
| `s` | sıralama sütunu |
| `/` | filtre |
| `+` / `-` | örnekleme aralığı |
| `g` | 60 sn / 5 dk grafik geçmişi |
| `F1` | yardım |
| `Esc` | kapat |
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
