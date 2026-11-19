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
- [ ] **T-010** `glm` (Hydrogen yerine: elle handoff beklemesin diye) — `ui/overview.rs` (göstergeler, motor çubukları, "—" durumları)
- [ ] **T-011** `glm` (Hydrogen yerine) — `ui/charts.rs` (4 zaman serisi, 60 sn/5 dk)
- [ ] **T-012** `glm` — `ui/procs.rs` (tablo, sıralama, `/` filtre)
- [ ] **T-013** `helper` — `ui/help.rs` + insta snapshot testleri
- [ ] **Kullanıcı:** gerçek kartta elle dene, ekran görüntüsünü Opus'a anlat veya yapıştır

## Faz 4 — CLI ve çıktı
- [ ] **T-014** `glm` — clap: `--interval`, `--card`, `--once`, `--json` (serde)
- [ ] **T-015** `helper` — `--record out.csv`

## Faz 5 — İleri (opsiyonel)
- [ ] **T-016** `rust-coder` — `source/pmu.rs` (perf_event_open, yetki yoksa sessizce devre dışı); motor doluluğu için PMU fdinfo'ya tercih edilir
- [ ] **T-017** `glm` — birden fazla GPU (`c` ile kart değiştirme), iGPU + dGPU senaryosu

## Faz 6 — Yayın
- [ ] **T-018** `helper` — README (kurulum, ekran görüntüsü, "neden nvtop değil"), `vhs` tape ile GIF
- [ ] **T-019** `helper` — GitHub Actions (fmt, clippy, test), release binary, AUR `PKGBUILD`
- [ ] **T-020** `glm` (Hydrogen yerine) — bağımsız code review: yalnızca `docs/REVIEW.md` yazar, kodu değiştirmez; Opus bulguları kartlara dönüştürür
