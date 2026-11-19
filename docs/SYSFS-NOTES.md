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

## Doğrulanmış değerler (Faz 0 sonrası doldurulacak)
| Öğe | Gerçek yol / biçim | Kernel |
|---|---|---|
| | | |
