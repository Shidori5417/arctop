# arctop — TUI tasarım spesifikasyonu (v2, "modern")

Amaç: btop/bottom kalitesinde, sakin ama canlı bir arayüz. Boş alan yok, her panel bilgi taşır, renkler anlamlıdır, tüm ekranlar 80x24'ten 200x60'a kadar düzgün ölçeklenir.

## 1. Tasarım dili
- **Kenarlıklar:** her panel `BorderType::Rounded`, kenarlık rengi `surface` (soluk). Odaklı/aktif panel kenarlığı `accent`.
- **Başlıklar:** panelin sol üstünde ` Başlık ` (bold, `text`), sağ üstünde ikincil bilgi (dim) — `Block::title(...).title(Line::from(..).right_aligned())`.
- **İç boşluk:** panellerde `Padding::horizontal(1)`.
- **Hiyerarşi:** büyük değer (bold, metrik rengi) → birim (dim) → ikincil satır (muted).
- **Unavailable:** `—` muted; nedeni yalnızca yardım penceresinde.
- **Unicode:** çubuklar `━` (dolu) + `─` (boş, surface rengi) veya `▰▱`; sparkline için ratatui `Sparkline` (bar set `NINE_LEVELS`); grafiklerde `Marker::Braille`. Emoji yok.

## 2. Palet (semantik tokenlar, `theme.rs`)
| Token | Truecolor | 256/16 renk geri dönüşü | Kullanım |
|---|---|---|---|
| `accent` | `#3D9BFF` (Arc mavisi, koyu zeminde okunur) | `Indexed(75)` / `LightBlue` | seçim, aktif sekme, odak |
| `accent_bg` | `#0071C5` | `Indexed(25)` / `Blue` | başlık rozeti, aktif sekme zemini |
| `text` | `Reset` | `Reset` | normal metin |
| `muted` | `#8B93A7` | `Indexed(246)` / `Gray` | ikincil metin, birimler |
| `surface` | `#3A4050` | `Indexed(238)` / `DarkGray` | kenarlıklar, boş çubuk |
| `row_alt` | `#1C2028` | `Indexed(234)` / `Reset` | zebra satır zemini |
| `ok` | `#4ADE80` | `Indexed(114)` / `Green` | düşük yük |
| `warn` | `#FBBF24` | `Indexed(220)` / `Yellow` | orta yük |
| `crit` | `#F87171` | `Indexed(203)` / `Red` | yüksek yük |
| `m_freq` | `#60A5FA` | `Indexed(75)` / `Cyan` | frekans serisi |
| `m_power` | `#FBBF24` | `Indexed(220)` / `Yellow` | güç |
| `m_temp` | `#FB923C` | `Indexed(209)` / `LightRed` | sıcaklık |
| `m_busy` | `#A78BFA` | `Indexed(141)` / `Magenta` | motor doluluğu |
| `m_vram` | `#2DD4BF` | `Indexed(79)` / `LightCyan` | VRAM |

- Renk modu çalışma zamanında bir kez seçilir: `NO_COLOR` (boş değilse) → renksiz (yalnızca bold/reverse/dim); `COLORTERM` `truecolor`/`24bit` → truecolor; aksi halde 256 renk. `--color <auto|always|never>` bayrağı gerekmez (v2 kapsamı dışı).
- Yük rengi (`load_color(pct)`): <50 `ok`, 50–80 `warn`, ≥80 `crit`. Sıcaklık için eşikler 60/80 °C.

## 3. Ortak çerçeve (her görünümde)
```
 ◆ arctop   1 Özet   2 Grafikler   3 Süreçler                Intel Arc A750 · i915   ● 1.0 s
╭─ ... görünüm gövdesi ...                                                                   ╮
╰────────────────────────────────────────────────────────────────────────────────────────────╯
 1-3 görünüm  s sırala  / filtre  +/- aralık  g pencere  F1 yardım  q çıkış
```
- **Başlık satırı (1 satır):** ` ◆ arctop ` rozeti (`accent_bg` zemin, bold). Sekmeler `N Ad` biçiminde; aktif sekme `accent_bg` zeminli "hap", diğerleri muted. Sağda cihaz adı + sürücü (muted) ve **canlılık noktası** `●`: her yeni snapshot'ta `ok` rengiyle yanar, ~0.5 aralık sonra `surface`'e söner (App'te `last_snapshot_at: Option<Instant>`), yanında örnekleme aralığı.
- **Altbilgi (1 satır):** kısayollar "çip" olarak: tuş `accent` bold, açıklama muted, aralarında iki boşluk. Görünüme göre bağlamsal: Süreçler'de `↑↓ seç  s sırala  / filtre`, Grafikler'de `g 60 sn/5 dk`. Filtre düzenlenirken altbilgi filtre girişine dönüşür: `/ firef▏  Enter uygula · Esc temizle`.
- Dar ekran (<70 sütun): cihaz adı ve çiplerin açıklamaları kısaltılır/atılır; panik yok.

## 4. Özet görünümü
```
 ╭ Frekans ─────── 2400 maks ╮╭ Güç ──────── limit 190 W ╮╭ Sıcaklık ───── fan 720 ╮╭ VRAM ─────────────────╮
 │ 2280 MHz                  ││ 59.0 W                   ││ 69 °C                  ││ 1.0 GiB               │
 │ cur 1200 · 300–2400       ││ %31 limit                ││ 755 mV                 ││ toplam bilinmiyor     │
 │ ▁▂▃▅▆▇██▇▆▇█████▇▇██      ││ ▂▃▃▄▅▅▆▇▆▅▅▆▇▇▆▅▆        ││ ▃▃▄▄▅▅▆▆▆▆▇▇▇▇        ││ ▅▅▅▅▅▅▅▅▆▆▆▆▆         │
 ╰───────────────────────────╯╰──────────────────────────╯╰────────────────────────╯╰───────────────────────╯
 ╭ Motorlar ─────────────────────────────── en yoğun %99 ╮╭ En çok kullananlar ──────────────────────────╮
 │ Render        ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  99% ││ blender      90%  ━━━━━━━━━━━━━━━     —      │
 │ Copy          ─────────────────────────────────    0% ││ mpv          60%  ━━━━━━━━━━         500 MiB │
 │ Video         ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  98% ││ firefox      20%  ━━━               300 MiB │
 │ Video-Enh.    ─────────────────────────────────    0% ││ steam        20%  ━━━               200 MiB │
 │ Compute       ─────────────────────────────────    0% ││ Xorg          5%  ━                 100 MiB │
 ╰───────────────────────────────────────────────────────╯╰──────────────────────────────────────────────╯
```
- **Stat kartları (üst sıra, 4 kart):** büyük değer (bold, metrik rengi) + birim (muted); ikinci satır bağlam (frekans: `cur · min–max`, `act==0` ise değer yerine `uykuda` muted; güç: limitin yüzdesi; sıcaklık: voltaj; VRAM: `toplam bilinmiyor` veya `%x / toplam`); alt satırlar: **sparkline** (`History` serisinin son `kart_genişliği` noktası, metrik rengi; `None` → 0). Sağ üst başlıkta: frekans `maks`, güç `limit`, sıcaklık `fan`, VRAM boş.
- **Motorlar paneli:** her sınıf bir satır: ad (sabit 12 sütun), çubuk (`load_color`), sağa hizalı yüzde. Hiç motor yoksa ortalanmış muted metin. Başlıkta `en yoğun %N`.
- **En çok kullananlar:** en yoğun 5 süreç (en yüksek motor yüzdesine göre; App'in sıralamasından bağımsız): ad, yüzde, mini çubuk, VRAM.
- **Yükseklik dağılımı:** stat kartları 6 satır sabit; alt sıra kalan alanın tamamı. Dikey alan 18 satırdan azsa sparkline satırları düşer. Genişlik < 100: kartlar 2x2; < 60: kartlar tek sütun ve alt sıra yalnızca Motorlar.

## 5. Grafikler görünümü
- 2x2 panel (dar ekranda 1x4 dikey). Her panel: başlık solda metrik adı, sağda **son değer** (bold, metrik rengi); alt kenarlıkta (Block `title_bottom`) `min · ort · maks` (muted).
- Seri metrik renginde, `Marker::Braille`; ızgara/eksen `surface` rengi; y ekseni etiketleri birimli (`2.4 GHz` biçimi: ≥1000 MHz → GHz bir ondalık).
- x ekseni etiketleri zaman: `-60 sn  -30 sn  şimdi` (5 dk penceresinde `-5 dk  -2.5 dk  şimdi`), aralıktan hesaplanır.
- Pencere göstergesi başlık satırında değil, panel grubunun üstünde tek satır: `Pencere: [60 sn] 5 dk` (aktif olan `accent` zeminli hap).

## 6. Süreçler görünümü
- Tek panel, rounded; başlık ` Süreçler `, sağ üstte `N / toplam`; alt kenarlık başlığında filtre durumu.
- Sütunlar: `PID` (7, sağa), `Ad` (esnek, min 12), `GPU` (mini çubuk 10 + yüzde; en yüksek motor yüzdesi), `Render`, `Copy`, `Video`, `VE`, `Compute` (her biri 7, sağa hizalı, 0.0 ise muted `·`), `VRAM` (9, sağa). 90 sütundan dar ekranda motor sütunları gizlenir, yalnızca `PID Ad GPU VRAM`.
- Başlık satırı: bold + `accent` alt çizgi yerine `surface` zeminli; aktif sıralama sütunu `accent` + `▼`/`▲`.
- **Seçim:** `↑/↓`, `j/k`, `PgUp/PgDn`, `Home/End` ile seçili satır (`accent_bg` zemin, bold). Seçim PID'e bağlı kalır (liste yeniden sıralanınca aynı PID seçili kalır; PID kaybolursa en yakın indeks). `TableState` ile kaydırma; sağda `Scrollbar` (yalnızca taşma varsa).
- Zebra satırlar (`row_alt`, yalnızca truecolor/256 modunda).
- Boş liste/filtre eşleşmesi yok: tablo yerine ortalanmış muted mesaj.

## 7. Yardım
- Ayrı ekran değil, **modal pencere**: önceki görünüm arkada çizilir, üstüne ortalanmış `Clear` + rounded panel (genişlik `min(76, alan-4)`, yükseklik içerik kadar). Başlık ` Yardım `, sağ üstte `F1/Esc kapat`.
- İki bölüm: **Kısayollar** (tuş çipleri, iki sütun) ve **Eksik metrikler** (`ad — neden`, muted neden). En altta tek satır not (root/PMU).
- `Esc` de yardımı kapatır (App değişikliği).

## 8. Uygulama sınırları
- Yeni crate yok (ratatui 0.29: `Sparkline`, `Scrollbar`, `TableState`, `BorderType::Rounded`, `Block::title_bottom`, `Padding` mevcut).
- Tüm renk/stil kararları `theme.rs` içinden; görünüm dosyalarında `Color::` yasak.
- Her görünüm için insta snapshot'ları (100x30, App::sample) yenilenir; 80x24, 60x20, 40x10 panik-yok testleri.
