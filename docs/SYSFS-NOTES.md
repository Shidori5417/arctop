# Donanım/sysfs notları

> Bu dosyadaki yollar **beklenen** değerlerdir. Kernel sürümüne ve sürücüye göre değişebilirler.
> Faz 0'da `scripts/discover-gpu.sh` çıktısı (`docs/hw/a750.txt`) ile doğrulanır ve bu dosya **gerçek** değerlerle güncellenir. Doğrulanmamış hiçbir yol koda girmez.

## Beklenenler (i915, DG2/A750)
- Frekans: `/sys/class/drm/card0/gt_{act,cur,min,max,boost,RP0,RPn}_freq_mhz`
- hwmon: `/sys/class/drm/card0/device/hwmon/hwmonN/` → `name` = `i915`; `energy1_input` (µJ), `power1_max`, `power1_crit`, `in0_input`; yeni kernellerde `fan1_input` gelebilir; sıcaklık olmayabilir
- PMU: `/sys/bus/event_source/devices/i915_<pci>` (dGPU) veya `i915`; olaylar `events/` altında
- fdinfo: `drm-driver: i915`, `drm-pdev`, `drm-client-id`, `drm-engine-{render,copy,video,video-enhance,compute}` (ns), `drm-total-local0` / `drm-resident-local0`

## Beklenenler (xe)
- Frekans: `device/tile0/gt0/freq0/{act,cur,min,max,rp0,rpn}_freq`
- hwmon: `name` = `xe`, `temp*_input` ve `energy*_input` (çip/paket ayrımı `*_label` ile)
- fdinfo: `drm-cycles-<sınıf>` ve `drm-total-cycles-<sınıf>` (doluluk = Δcycles / Δtotal_cycles)

## Doğrulanmış değerler
Kaynak: `docs/hw/a750.txt` (2026-11-19 05:32). Sayısal örnekler bu dosyadan; t0/t1 farkı `tests/fixtures/a750/` (canlı, 2 sn arayla).

| Öğe | Gerçek yol / biçim | Kernel |
|---|---|---|
| Kart dizini | `/sys/class/drm/card1` (vendor `0x8086`, device `0x56a1`, PCI `0000:03:00.0`) | 7.3.4-1-cachyos |
| Sürücü | `/sys/class/drm/card1/device/driver` symlink'i → `i915` | 7.3.4-1-cachyos |
| Frekans (i915) | `/sys/class/drm/card1/gt_{act,cur,min,max,boost,RP0,RP1,RPn}_freq_mhz`, düz tamsayı, MHz. Örnek: act 0, cur 600, min 300, max 2400, boost 2400, RP0 2400, RP1 600, RPn 300 | 7.3.4-1-cachyos |
| Frekans (xe) | Yok. `device/tile*/gt*/freq*` araması boş | 7.3.4-1-cachyos |
| hwmon dizini | `/sys/class/drm/card1/device/hwmon/hwmon2/`, `name` = `i915` | 7.3.4-1-cachyos |
| `energy1_input` | µJ, kümülatif sayaç. Örnek 467302016235 | 7.3.4-1-cachyos |
| `power1_max` | µW. Örnek 190000000 (190 W) | 7.3.4-1-cachyos |
| `power1_rated_max` | µW. Değer 0 | 7.3.4-1-cachyos |
| `power1_max_interval` | Var (canlı `ls`). `*_max` glob'una uymadığı için a750.txt'de yazdırılmadı, birimi doğrulanmadı | 7.3.4-1-cachyos |
| `power1_crit` | Yok | 7.3.4-1-cachyos |
| `in0_input` | Gerilim, hwmon ABI'ye göre mV (doğrulanmadı). a750.txt'de 0; canlı fixture'da t0 825, t1 755 | 7.3.4-1-cachyos |
| `temp1_input` | m°C (hwmon ABI). Örnek 48000 = 48,0 °C. `temp1_label` yok | 7.3.4-1-cachyos |
| `fan1_input` | RPM. Örnek 539 | 7.3.4-1-cachyos |
| PMU adı | `/sys/bus/event_source/devices/i915_0000_03_00.0`, `type` = 14 (`i915` değil) | 7.3.4-1-cachyos |
| PMU olayları | 26 olay, `events/<ad>`; birim `events/<ad>.unit`. `actual-frequency`, `requested-frequency` (birim `M`, MHz olarak yorumlanır); `rc6-residency`, `software-gt-awake-time` (ns); `interrupts` (birimsiz); `{bcs0,ccs0,rcs0,vcs0,vcs1,vecs0,vecs1}-{busy,sema,wait}` (ns) | 7.3.4-1-cachyos |
| fdinfo anahtarları | `drm-driver: i915`, `drm-client-id`, `drm-pdev: 0000:03:00.0`. Bellek: `drm-{total,shared,active,resident,purgeable}-{system0,local0,stolen-local0}` | 7.3.4-1-cachyos |
| fdinfo bellek birimi | `<n> KiB` (örn. `25088 KiB`), ama değer 0 iken birimsiz `0`. Ayrıştırıcı iki biçimi de kabul etmeli | 7.3.4-1-cachyos |
| fdinfo motor | `drm-engine-{render,copy,video,video-enhance,compute}`: `<n> ns` (örn. `0 ns`) | 7.3.4-1-cachyos |
| fdinfo kapasite | `drm-engine-capacity-{video,video-enhance}`: birimsiz tamsayı (örn. `2`) | 7.3.4-1-cachyos |
| VRAM toplamı | sysfs'de girdi yok (`vram|tile|lmem` araması boş). fdinfo `drm-total-local0` istemci başınadır, toplam değildir | 7.3.4-1-cachyos |

### Beklenenden sapmalar
- Kart `card1`, `card0` değil.
- fdinfo bellek değerleri birimli: `<n> KiB`. Değer 0 ise birimsiz `0`.
- `drm-engine-capacity-video` ve `drm-engine-capacity-video-enhance` var; birimsiz.
- `in0_input` a750.txt'de 0, ama canlı t0/t1'de 825 ve 755. Sıfır tek örnekti, sabit değil; test sabit 0 varsaymamalı.
- `temp1_input` mevcut (48000 m°C), `temp1_label` yok.
- `power1_rated_max` = 0.
- `power1_crit` yok (Beklenenler'de vardı).
- `gt_RP1_freq_mhz` var (Beklenenler'de yoktu).
- `fan1_input` var (539 RPM).
- VRAM toplamı için sysfs girdisi yok.
- xe frekans yolu yok (sürücü i915).
- PMU adı `i915_<pci>` biçiminde (`i915_0000_03_00.0`).
- fdinfo'da ek anahtarlar var: `drm-client-id`, `drm-pdev`, `*-system0`, `*-stolen-local0`.
- `driver.name` fixture'a özgü: script `driver` symlink'inin hedefini yazar. Gerçek sysfs'te böyle bir dosya yok.
- t0 → t1 (2 sn) değişenler: `energy1_input` 469690564208 → 469796980224; `in0_input` 825 → 755; `gt_cur_freq_mhz` 750 → 600; fdinfo `drm-active-local0` 0 → 8 MiB (pid 1815, client 22). Δ testleri bu alanları kullanabilir.
- fdinfo örneklerinin tamamı Hyprland'a (pid 1815) ait. Oyun ya da GPU uygulaması açık değildi.
