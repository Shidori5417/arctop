# arctop — Yol haritası ve görev dağılımı

Atamalar önerilir; Opus `CLAUDE.md` karar ağacına göre değiştirebilir. 🔒 = arayüz kartı (paralel işlerden önce main'e alınır).

## Faz 0 — Keşif
- [x] **Kullanıcı:** (Opus çalıştırdı, masaüstü boştayken — motor sayaçları 0; yük altında tekrar alınabilir) bir oyun ya da glxgears açıkken `scripts/discover-gpu.sh > docs/hw/a750.txt`, ardından birkaç saniye arayla `scripts/discover-gpu.sh --fixture tests/fixtures/a750/t0` ve `... t1`
- [x] **T-000** `helper` — `docs/hw/a750.txt` çıktısını oku ve `docs/SYSFS-NOTES.md` içindeki "Doğrulanmış değerler" tablosunu doldur; beklenenlerden sapmaları listele

## Faz 1 — İskelet ve arayüzler
- [x] **T-001** `helper` — cargo workspace, iki crate, bağımlılıklar (ARCHITECTURE §7), boş modüller, CI iskeleti
- [x] **T-002** 🔒 `rust-coder` — `SysFs` (§1), `model` (§4), `MetricSource` trait'i (§3), hata tipleri; SysFs testleri
- [x] **T-003** `glm` — `discover` (§2) + `pci_ids` tablosu; fixture testi

## Faz 2 — Metrik kaynakları (paralel)
- [x] **T-004** `glm` — `source/freq.rs` (i915 + xe yolları)
- [x] **T-005** `glm` — `source/hwmon.rs` (label eşleme, energy Δ → W)
- [x] **T-006** `rust-coder` — `source/fdinfo.rs` (client-id tekilleştirme, i915 ns ve xe cycles formülleri, pdev filtresi, süreç temizliği; performans hedefi SPEC'te)
- [x] **T-007** `rust-coder` — `sampler` + `history` (§5), kontrol kanalı, testler
- [x] **T-008** `helper` — `examples/dump.rs`: gerçek sistemde 3 snapshot alıp debug-print eder (kullanıcı çalıştırıp doğrular)

- [x] **T-021** (açık madde): `vram_total` hiçbir kaynakça doldurulmuyor ("örneklenmedi" görünüyor); sysfs girdisi yok, neden "sysfs'de yok" olmalı (Faz 3 öncesi küçük kart, `helper`)

## Faz 3 — TUI
- [x] **T-009** 🔒 `rust-coder` — `App` durumu, olay döngüsü, görünüm geçişi, `render` imzaları, `theme.rs`, terminal restore (panic hook dahil)
- [x] **T-010** `glm` (Hydrogen yerine: elle handoff beklemesin diye) — `ui/overview.rs` (göstergeler, motor çubukları, "—" durumları)
- [x] **T-011** `glm` (Hydrogen yerine) — `ui/charts.rs` (4 zaman serisi, 60 sn/5 dk)
- [x] **T-012** `glm` — `ui/procs.rs` (tablo, sıralama, `/` filtre)
- [x] **T-013** `helper` — `ui/help.rs` + insta snapshot testleri
- [x] **Kullanıcı:** (Opus pty üzerinden gerçek kartta denedi: tüm görünümler, sıralama, F1, q) gerçek kartta elle dene, ekran görüntüsünü Opus'a anlat veya yapıştır

## Faz 4 — CLI ve çıktı
- [x] **T-014** `glm` — clap: `--interval`, `--card`, `--once`, `--json` (serde)
- [x] **T-015** `helper` — `--record out.csv`

## Faz 5 — İleri (opsiyonel)
- [ ] **T-016** `rust-coder` — `source/pmu.rs` (perf_event_open, yetki yoksa sessizce devre dışı); motor doluluğu için PMU fdinfo'ya tercih edilir
- [ ] **T-017** `glm` — birden fazla GPU (`c` ile kart değiştirme), iGPU + dGPU senaryosu

## Faz 6 — Yayın
- [x] **T-018** `helper` — README (kurulum, ekran görüntüsü, "neden nvtop değil"), `vhs` tape ile GIF
- [x] **T-019** `helper` — GitHub Actions (fmt, clippy, test), release binary, AUR `PKGBUILD`
- [x] **T-020** `glm` (Hydrogen yerine) — bağımsız code review: yalnızca `docs/REVIEW.md` yazar, kodu değiştirmez; Opus bulguları kartlara dönüştürür

## Faz 6b — Review düzeltmeleri
- [x] **T-022** `rust-coder` — fdinfo: xe anahtar adları/kapasite, bellek Option, global client-id tekilleştirme, gerçek yol benchmark (R-01..R-05)
- [x] **T-023** `helper` — grafik başlığı, küçük terminal testleri, yardımda tekrar, JSON yuvarlama (R-08, R-09)
- [x] **T-024** `rust-coder` — fdinfo taramasını 5 ms altına indir (canlı: 6-7 ms, 347 süreç)
- R-06 (hwmon xe label'ları) ve xe doğrulaması: xe donanımı bulunana kadar açık (ADR-007)

## Faz 7 — TUI v2 (modernleştirme; yalnızca Sonnet/Haiku)
- [x] **T-025** 🔒 `rust-coder` — palet, `ui::widgets`, başlık/altbilgi, modal yardım çerçevesi, App seçim/canlılık durumu
- [x] **T-026** `rust-coder` — Özet: stat kartları + sparkline, motorlar, en çok kullananlar
- [x] **T-027** `rust-coder` — Grafikler: metrik renkleri, birimli eksenler, zaman etiketleri, min/ort/maks
- [x] **T-028** `rust-coder` — Süreçler: seçim, kaydırma çubuğu, GPU mini çubuğu, duyarlı sütunlar
- [x] **T-029** `helper` — Yardım modal içeriği
- [x] **T-030** `rust-coder` — Özet cilası: 5 motor, Etkinlik paneli
- [x] **T-031** `helper` — yardım çip genişliği, ölü kod, README ekran görüntüleri
