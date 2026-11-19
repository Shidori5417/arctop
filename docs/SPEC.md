# arctop — Gereksinimler

## Hedef
Intel Arc kullanıcısının tek komutla GPU'sunun ne yaptığını görmesi: hangi frekansta çalıştığı, ne kadar güç çektiği, hangi motorların meşgul olduğu ve hangi sürecin GPU'yu kullandığı.

## Platform
- Linux 6.8+ (CachyOS ile test edilir), `i915` (öncelikli) ve `xe` sürücüleri
- Root gerekmez. PMU için `perf_event_paranoid <= 0` veya `CAP_PERFMON` gerekir; bunlar yoksa fdinfo'ya düşülür
- **Salt okunur:** sysfs'e hiçbir şey yazılmaz (overclock ve fan kontrolü kapsam dışıdır)

## Metrikler (MVP)
| Metrik | Kaynak (öncelik sırasıyla) | Not |
|---|---|---|
| Frekans: act / cur / min / max / boost | sysfs gt_*freq (i915), tile/gt/freq0 (xe) | MHz |
| Güç | hwmon `energy*_input` farkı / Δt | W; `power*_input` varsa doğrudan |
| Güç limiti | hwmon `power*_max` / `power*_crit` | |
| Sıcaklık | hwmon `temp*_input` | i915'te kernel sürümüne göre olmayabilir |
| Fan | hwmon `fan*_input` | RPM, olmayabilir |
| Voltaj | hwmon `in0_input` | mV |
| Motor doluluğu | PMU (varsa), yoksa tüm fdinfo istemcilerinin toplamı | % |
| Süreç listesi | `/proc/*/fdinfo` (drm-*) | PID, ad, motor %, VRAM; yalnızca okunabilen süreçler |
| VRAM | fdinfo `drm-resident-local0` toplamı, sysfs varsa toplam | MiB |

Okunamayan her metrik `Unavailable(reason)` olarak taşınır. UI "—" gösterir, nedeni F1 ekranında yazar.

## Görünümler
1. **Özet:** kart adı, sürücü, PCI; frekans, güç, sıcaklık ve fan göstergeleri; motor çubukları
2. **Grafikler:** frekans, güç, sıcaklık ve toplam doluluk için zaman serileri (60 sn / 5 dk arasında geçiş)
3. **Süreçler:** sıralanabilir tablo (motor %, VRAM, PID, ad) ve `/` ile filtre
4. **Yardım (F1):** kısayollar ve eksik metriklerin nedenleri

## Kısayollar
`1-3` görünüm · `Tab` sonraki görünüm · `s` sıralama sütunu · `/` filtre · `+`/`-` örnekleme aralığı · `g` 60 sn/5 dk · `c` birden fazla GPU varsa kart değiştir · `F1` yardım · `q` çıkış

## CLI
```
arctop [--interval MS] [--card N]
arctop --once [--json]         # tek ölçüm (Δ gerektiren metrikler için iki örnek alır)
arctop --record out.csv        # TUI açıkken CSV'ye kaydet (Faz 4)
```

## Performans hedefleri
- 1 sn aralıkla kendi CPU kullanımı < %1, RSS < 15 MB
- fdinfo taraması 500 süreçte < 5 ms

## Kapsam dışı (v1)
Overclock, fan eğrisi, NVIDIA/AMD desteği, GUI.
