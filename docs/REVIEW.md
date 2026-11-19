# Bağımsız kod incelemesi (T-020)

İncelenen: `fa2b17e` (task/T-020 dalı, Faz 3+4 kodunun tamamı: `crates/**`, ~5800 satır).
Yöntem: dokümanlar + tüm kaynak kod okundu; `cargo fmt --check`, `clippy -D warnings`, `cargo test` (115/116, 1 ignored bench) çalıştırıldı; yoksayılan performans testi release modda koşuldu; xe davranış iddiaları çekirdek belgeleriyle (docs.kernel.org, xe_drm_client.c) doğrulandı. Kod değiştirilmedi.

Bulgular önem sırasına göredir. "Doğrulanmalı" = gerçek xe donanımı ya da canlı sistem gerektirir.

### R-01 [yüksek] xe fdinfo motor anahtarları sınıf adı değil, örnek adıdır; xe'de tüm motor metrikleri boş kalır

- `crates/arctop-core/src/source/fdinfo.rs:42` (`class_index`), `crates/arctop-core/src/source/fdinfo.rs:147-155`
- Çekirdek belgesi (Xe DRM usage stats): xe `drm-cycles-<ad>` / `drm-total-cycles-<ad>` için yalnızca `rcs, ccs, bcs, vcs, vecs, "other"` adlarını yazar; `render/copy/video/...` sınıf adları i915'e özgüdür. `class_index` yalnızca 5 sınıf adını eşlediğinden gerçek xe donanımında her `drm-cycles-*` satırı sessizce atılır: `engines` hep boş, süreç başına doluluk hep `—` çıkar. SYSFS-NOTES'taki "drm-cycles-<sınıf>" beklentisi bu nedenle yanlış şekildedir; ADR-007'nin istediği `// DOĞRULANMADI (xe)` yorumu da fdinfo.rs'in xe dallarında yok (freq.rs:86'da var).
- Düzeltme: `Driver::Xe` dalında örnek adlarını sınıfa eşleyen ayrı bir harita ekle (`rcs→Render, bcs→Copy, vcs→Video, vecs→VideoEnhance, ccs→Compute`, "other" yoksayılır); SYSFS-NOTES xe bölümünü ve sabiti çekirdek belgesiyle güncelle; xe dallarına `// DOĞRULANMADI (xe)` yorumu koy.

### R-02 [yüksek] xe doluluk formülü `drm-engine-capacity`'yi paydada kullanmıyor

- `crates/arctop-core/src/source/fdinfo.rs:191-198` (`Counters::pct`, `Driver::Xe` dalı)
- xe çekirdeği `run_ticks`'i sınıfın TÜM örnekleri üzerinden toplar, `drm-total-cycles` ise tek bir GPU zaman damgasıdır; sınıf kapasitesi >1 iken (`drm-engine-capacity-vcs: 2`, `ccs: 4` örnekleri belgede var) meşgul çevrim `capacity × Δtotal`'e kadar çıkabilir. Formüldeki `dc/dt` 2 örnekli bir sınıfın gerçek %50 doluluğunu %100 raporlar. i915 dalı capacity'yi bölüyor; xe dalı bölmüyor (capacity alanı ayrıştırılıyor ama kullanılmıyor).
- Düzeltme: xe dalını da i915 gibi `d as f64 / (dt * capacity.max(1)) * 100` hesabına çevir; 3 örnekli bir xe fixture testi ekle.

### R-03 [orta] xe bellek anahtarları ayrıştırılmıyor; eksik/bozuk `resident-local0` sessizce 0'a düşüyor

- `crates/arctop-core/src/source/fdinfo.rs:136` (`resident-local0 = parse_memory(val).unwrap_or(0)`)
- Çekirdek belgesine göre xe bellek bölgeleri `drm-resident-{system,gtt,vram0,stolen}`'dır; `local0` i915'e özgüdür. xe'de anahtar hiç bulunmayacağından `vram_used` yanıltıcı biçimde `Value(0)` ("0 MiB") olur, `Unavailable` değil. Aynı `unwrap_or(0)` i915'de de beklenmeyen birim/biçimde sessiz sıfır üretir.
- Düzeltme: `FdInfo` bellek alanını `Option` yap; sürücüye göre xe'te `resident-vram0` (yoksa `Unavailable`) oku; hiç bellek anahtarı yoksa `vram_used`'i `Unavailable("fdinfo'da yok")` olarak işaretle.

### R-04 [orta] fork ile paylaşılan DRM fd'i aynı client-id'yi iki PID'de gösterir; toplam doluluk ve VRAM çift sayılır (doğrulanmalı)

- `crates/arctop-core/src/source/fdinfo.rs:332-337` (`clients` tekilleştirmesi pid sınırlı), `crates/arctop-core/src/source/fdinfo.rs:348-356,362,371`
- Fork'ta kalan çocuk aynı açık dosya tanımını (ve `drm-client-id`'yi) paylaşır: aynı sayaçlar iki PID altında görünür. `(pid, client_id)` anahtarı (ARCHITECTURE §3'e harfiyen uyumlu) iki kayıt da tuttuğundan `tot_pct` (100'e kırpılsa bile şişer) ve `tot_vram` o istemciyi iki kez sayar; süreç tablosunda özdeş iki satır görünür. intel-gpu-top toplamları client-id üzerinden global tekilleştirir. Gerçek masaüstünde sıklığı doğrulanmalı.
- Düzeltme: süreç satırları pid bazlı kalabilir; `tot_pct`/`tot_vram` toplamlarını `(pdev, client_id)` üzerinden global tekilleştir.

### R-05 [orta] "500 süreçte < 5 ms" hedefi, gerçek sistemde kullanılan kod yolunu ölçmeyen bench ile doğrulanıyor (doğrulanmalı)

- `crates/arctop-core/tests/fdinfo.rs:174-209` (bench ağacında `/proc/N/fd` yok → fixture geri düşme yolu), `crates/arctop-core/src/source/fdinfo.rs:227-259` (`candidate_paths`)
- Gerçek sistemde `/proc/N/fd` vardır: süreç başına bir `read_dir` + HER fd için bir `readlink` yapılır. Maliyet süreç sayısından çok sistemdeki toplam açık fd sayısıyla ölçeklenir (tarayıcı/sunucu süreçleriyle on binlerce `readlink`); 5 ms hedefi fd-yoğun masaüstünde aşılabilir. Bench yalnızca `fdinfo/*` tarama yolunu ölçer (release'ta geçiyor) — hedef bu yolda doğrulanmış sayılır.
- Düzeltme: bench ağacına `/proc/N/fd` + ADR-004 `.name` dosyaları ekle, gerçek yolu ölç; geliştirme makinesinde canlı süreç sayısıyla karşılaştır. Aşılırsa yalnızca DRM fd'lerinin fdinfo'sunu açan tarama değerlendirilsin.

### R-06 [düşük] hwmon girdi seçimi: sabit indeksler ve xe label tercihleri doğrulanmadı (doğrulanmalı)

- `crates/arctop-core/src/source/hwmon.rs:96-97,135-136` (`fan1_input`, `in0_input` sabit), `crates/arctop-core/src/source/hwmon.rs:196-205` (`power1_*` sabit), `crates/arctop-core/src/source/hwmon.rs:179-186` (`"card"`/`"pkg"` tercihleri)
- energy/temp/power indeksleri `*_label` taramasıyla bulunurken fan, gerilim ve güç limiti `1` indeksine sabitlenmiş; hwmon ABI'de indeks başka olabilir → veri varken sessizce `Unavailable` olur (yanlış veri üretmez). xe hwmon'da `"card"` (kart toplamı) ve `"pkg"` tercihlerinin gerçekten bu anlama geldiği doğrulanmalı; xe'de kart toplamı yerine tek bir GT'nin kanalı seçilirse güç metrikası düşük kalır.
- Düzeltme: `scan_inputs`'ı `fan`/`in` için de kullan; xe donanımında label anlamlarını SYSFS-NOTES'a kaydet; sabit kalanları bilinçli olarak belgele.

### R-07 [düşük] `record.rs` üretim kodunda `std::fs` kullanıyor; kural istisnası kayıtlı değil

- `crates/arctop/src/record.rs:3,38`
- Kural "std::fs yalnızca sysfs.rs (ve izinli testler)" (AGENTS.md, T-020 odağı 5). CSV yazma bir sysfs *okuması* olmadığından SysFs soyutlamasının ruhuna aykırı değil, ancak harfiyen kural dışıdır ve sysfs.rs'teki "tek kullanım noktası" yorumu (sysfs.rs:3) artık dar kapsamlı.
- Düzeltme: DECISIONS'a tek satırlık bir ADR ekle ("bin crate'teki CSV yazıcı std::fs kullanabilir; SysFs salt-okunur kalır") veya dosya oluşturmayı küçük bir yardımcının arkasına al; sysfs.rs:3 yorumunu buna göre güncelle.

### R-08 [düşük] "Toplam doluluk" grafik serisi aslında en yoğun tek motoru gösteriyor

- `crates/arctop-core/src/history.rs:90-97` (`busy_total` = motorların `max`'ı), `crates/arctop/src/ui/charts.rs:76-84` (başlık "Toplam doluluk (%)")
- SUM, farklı motorlar eşzamanlı meşgulken meşruiyetle %100'ü aşacağından `max` seçimi savunulabilir (ve kod belgelenmiş); ancak SPEC'in "toplam doluluk" sözcüğü ve grafik başlığı kullanıcıya "tüm GPU'nun doluluğu" izlenimi verir: render %100 + video %100 iken seri %100 gösterir, kopya %0'ı gizler.
- Düzeltme: ya grafiği "En yoğun motor (%)" olarak yeniden adlandır ya da bu yorumu SPEC/Grafikler bölümüne not olarak ekle.

### R-09 [düşük] 80x24 altı terminal testi yalnızca Özet görünümünde var

- `crates/arctop/src/ui/overview.rs:377-382` (20x4 testi var); `charts.rs`, `procs.rs`, `help.rs` testleri ≥80x24
- T-020 odağı 4 "80x24 ve daha küçük terminal" diyor; Grafikler/Süreçler/Yardım 20x4 gibi uç boyutlarda çizim testinden geçmiyor. Panik riski düşük: ratatui 0.29 `Chart::layout` sıfır alan için erken `None` döner (kaynak kodundan doğrulandı), `Table` kısıt çözücüyle daraltır; yine de düşük genişlikte kalan alanlar panik atmamalı garanti edilmeli.
- Düzeltme: `ui/mod.rs` düzeyinde `draw`'ı 20x4 (ve 0 genişlik dışı uçlar) ile dökecek ortak bir duman testi ekle; üç görünümü de kapsasın.

## Gözden geçirildi, sorun yok

- **i915 fdinfo matematiği:** Δns/(Δt·capacity), pid içi client-id tekilleştirme, kaybolan istemci temizliği, `checked_sub` ile sayaç geriye gidişinde sessiz atlama, pdev filtresi — sentetik + a750 fixture testleriyle doğrulu.
- **Birimler:** KiB/MiB/GiB→bayt, ΔµJ→W, µW→W, m°C→°C doğru; "0" birimsiz fdinfo bellek değeri de destekli (SYSFS-NOTES sapması işlenmiş).
- **hwmon sayaç sıfırlanması:** "sayaç sıfırlandı" raporlanıyor, referans yenileniyor, sonraki örnek kendini toparlıyor (testi var).
- **Örnekleyici/iletici kapanışı:** Receiver düşünce F `Stop` yollar, `stop()` S→F sırayla join; yavaş tüketici testte doğrulandı; `Drop` çift çağrı zararsız; A kanalı bounded(2) + en eskiyi atma sapma biriktirmiyor.
- **Terminal geri yükleme:** panic hook zincirleme + `TerminalGuard` + idempotent `restore_terminal`; `init_terminal` yarım kalan yolu raw mode'u geri kapatıyor; arka plan thread'i panikse B kanalı kapanıp ana döngüye "örnekleyici kapandı" hatasıyla temiz çıkılıyor.
- **Süreç yarışları:** fdinfo/comm okunurken süreç ölürse sessiz atlama, comm "?" düşüyor; `/proc/N/fd` izin hatasında süreç atlanıyor (SPEC'in "yalnızca okunabilen süreçler" kuralı).
- **Kurallar:** `unsafe` yok; `unwrap/expect` yalnızca `#[cfg(test)]` bloklarında (çözümlemesi yapıldı); core `#![warn(missing_docs)]`; bağımlılıklar ADR-005/§7 ile aynı; xe frekans yolu ADR-007 gereği `DOĞRULANMADI` işaretli ve üçüncü taraf gözlemlerle uyumlu.
- **Doğrulama:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` temiz; 115/116 test geçti (1 ignored); `sample_500_procs_under_5ms` release modda geçti (fixture yolu; bkz. R-05).
