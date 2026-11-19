# Mimari kararlar (ADR)

## ADR-001: tokio yok, tek örnekleyici thread
Tüm işler kısa ve senkron dosya okumalarından oluşuyor. Async runtime gereksiz yük getirir; std::thread ve crossbeam yeterli.

## ADR-002: Tüm okumalar SysFs soyutlamasından geçer
Ajanların ve CI'ın donanım olmadan test yazabilmesi için. Fixture'lar gerçek makineden alınır.

## ADR-003: Salt okunur
v1 sysfs'e hiçbir şey yazmaz. Güvenlik ve root gereksinimi açısından sade kalır.
