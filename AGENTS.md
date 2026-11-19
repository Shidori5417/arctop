# AGENTS.md — Uygulayıcı ajan kuralları
(Claude Sonnet/Haiku alt ajanları, OpenCode GLM 5.3 ve Antigravity Hydrogen için ortak kurallar)

Bu projede **uygulayıcısın**. Orkestratör (Opus) sana bir görev kartı verir. Kartın dışına çıkmazsın.

## Başlamadan önce
1. Görev kartını oku: `.tasks/T-xxx.md`.
2. Kartın **"Okunacaklar"** kısmındaki dosyaları ve bölümleri oku. Daha fazlasını okuma.
3. Kartın dalında olduğunu doğrula: `git branch --show-current` çıktısı `task/T-xxx` olmalı. (Claude alt ajanları ana ağaçta çalışabilir.)

## Kurallar
- Yalnızca kartın **"Sahip olunan dosyalar"** listesindeki dosyaları değiştir. Başka bir dosyada değişiklik gerekirse kendin yapma; raporda "Gerekli dış değişiklik" başlığı altına yaz.
- Kartta **"Sabit arayüz"** olarak verilen imzaları değiştirme.
- Yeni bağımlılık (crate) ekleme. Gerekiyorsa raporda öner.
- Rust kuralları:
  - edition 2024, `cargo fmt` uygulanmış olmalı, `cargo clippy --all-targets -- -D warnings` temiz çıkmalı
  - `unwrap()`/`expect()` yalnızca testlerde ya da kanıtlanmış değişmezlerde kullanılır (yanına yorum yaz)
  - hata tipleri: kütüphanede `thiserror`, binary'de `anyhow`
  - `unsafe` yalnızca kart izin veriyorsa ve `// SAFETY:` yorumuyla birlikte kullanılır
  - her public öğeye doc yorumu, her modüle en az bir test
- Emin olmadığın donanım veya protokol davranışını uydurma. Raporda "Açık soru" olarak yaz.

## Bitirince
1. `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` komutları temiz çıkmalı.
2. Commit mesajı: `T-xxx: <kısa açıklama>`
3. `.tasks/done/T-xxx.report.md` dosyasını yaz (en fazla 15 satır) ve commit et:

```markdown
# T-xxx rapor
- Durum: tamam | kısmi | başarısız
- Değişen dosyalar: ...
- Yapılan: (2-4 madde)
- Testler: X geçti / Y toplam
- Gerekli dış değişiklik: (yoksa "yok")
- Açık sorular: (yoksa "yok")
```
