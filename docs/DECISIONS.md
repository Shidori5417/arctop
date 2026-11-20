# Mimari kararlar (ADR)

## ADR-001: tokio yok, tek örnekleyici thread
Tüm işler kısa ve senkron dosya okumalarından oluşuyor. Async runtime gereksiz yük getirir; std::thread ve crossbeam yeterli.

## ADR-002: Tüm okumalar SysFs soyutlamasından geçer
Ajanların ve CI'ın donanım olmadan test yazabilmesi için. Fixture'lar gerçek makineden alınır.

## ADR-003: Salt okunur
v1 sysfs'e hiçbir şey yazmaz. Güvenlik ve root gereksinimi açısından sade kalır.

## ADR-004: Fixture kökü gerçek yol ağacını aynalar
Fixture'lar `tests/fixtures/<makine>/tN/` altında `/sys/...` ve `/proc/...` yollarını birebir taşır; `SysFs::with_root` bu dizini kök alır. Kaynaklar her zaman mutlak gerçek yollarla çalışır, test/gerçek ayrımı yalnızca kökte yapılır. Sembolik bağlar `<bağ>.name` dosyasıyla temsil edilir.

## ADR-005: Bağımlılık sürümleri (T-001)
ratatui 0.29, crossterm 0.28, clap 4, thiserror 2, crossbeam-channel 0.5, serde 1, insta 1. Sürüm yükseltmesi ayrı bir kartla yapılır.

## ADR-006: Kart yolu donanıma göre keşfedilir
Geliştirme makinesinde A750 `card1` olarak görünüyor (card0 değil). Hiçbir kaynak kart numarası varsaymaz; `Device.card` keşiften gelir. fdinfo bellek değerleri `<n> KiB` biçimindedir, ayrıştırıcı birim son ekini işlemelidir.

## ADR-007: xe yolları doğrulanmadan kodda yer alabilir (işaretli)
Geliştirme makinesinde xe yok. xe için ARCHITECTURE §3'teki beklenen yollar koda girebilir, ancak `// DOĞRULANMADI (xe)` yorumuyla ve yalnızca `Driver::Xe` dalında. xe donanımında doğrulanınca yorum kaldırılır ve SYSFS-NOTES güncellenir.

## ADR-008: Örnekleyici iki thread kullanır (ADR-001'i daraltır)
Örnekleme tek thread'de kalır; yanına yalnızca kanal aktaran küçük bir iletici thread eklenir. Gerekçe: "kanal doluysa en eskiyi at" için gönderen bir Receiver klonu tutmak zorunda, bu da UI Receiver'ı düştüğünde kanalın kapanmasını engeller. İletici, kullanıcı kanalı kapanınca `Stop` gönderir. tokio yine yok.

## ADR-009: `sample_once` probe çağırmaz
`Sampler::spawn` probe'u kendisi çağırır; `sample_once` çağıran tarafın `probe` yapmasını bekler (`--once` uygulaması, T-014, bunu yapmalı).

## ADR-010: `record.rs` için `std::fs` istisnası (REVIEW R-07)
"Doğrudan `std::fs` yasak" kuralı **okuma** içindir (SysFs testlenebilirliği). `--record` kullanıcı dosyasına **yazar**; bu binary crate'te `record.rs` içinde `std::fs::File` kullanımı serbesttir. sysfs'e yazma yasağı (ADR-003) aynen geçerlidir.

## ADR-011: fdinfo istemcileri global tekilleştirilir (REVIEW R-04)
fork ile paylaşılan fd aynı `drm-client-id`'yi birden fazla PID'de gösterir (geliştirme makinesinde 8 PID'de görüldü). Sayaçlar `(drm-pdev, drm-client-id)` anahtarıyla bir kez sayılır; paylaşılan istemci en küçük PID'e atanır. ARCHITECTURE §3'teki `(pid, client_id)` anahtarının yerini alır.

## ADR-012: fdinfo tarama önbelleği, `RESCAN_EVERY = 5` (T-024)
DRM fd'si olmayan süreçler her 5 örnekte bir yeniden taranır; DRM süreçlerinde yalnızca bilinen fd'ler okunur. Canlı tarama 6–7 ms → ~2 ms. Bedel: GPU'yu sonradan açan eski bir süreç en geç 5 örnek sonra görünür. 8–10'a çıkarmak sentetik payı artırır ama gecikmeyi uzatır; gerek görülmedi.

## ADR-013: TUI v2 tasarım dili (docs/TUI-DESIGN.md)
Rounded paneller, semantik renk tokenları (`theme::Tok`), truecolor/256/NO_COLOR modları, modal yardım, süreç tablosunda PID'e bağlı seçim. Görünüm dosyalarında doğrudan `Color::` kullanımı yasak; tüm stil `theme` ve `ui::widgets` üzerinden. Bu fazda yalnızca Claude Sonnet (`rust-coder`) ve Haiku (`helper`) kullanılır (kullanıcı isteği).
