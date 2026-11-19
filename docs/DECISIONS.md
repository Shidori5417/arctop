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
