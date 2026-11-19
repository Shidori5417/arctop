#!/usr/bin/env bash
# arctop Faz 0: Intel GPU sysfs/fdinfo keşfi. Root gerekmez, hiçbir şey yazmaz (yalnızca --fixture hedefine).
#   scripts/discover-gpu.sh > docs/hw/a750.txt          # okunabilir rapor
#   scripts/discover-gpu.sh --fixture tests/fixtures/a750/t0   # testler için ilgili dosyaların kopyası
# İpucu: fdinfo verisinin görünmesi için bir oyun ya da glxgears açıkken çalıştır.
set -u
FIX=""
[ "${1:-}" = "--fixture" ] && FIX="${2:?--fixture DIZIN}"

copy() { # copy <mutlak yol>: değeri fixture'a kopyalar
  [ -n "$FIX" ] || return 0
  local src="$1" dst="$FIX$1"
  mkdir -p "$(dirname "$dst")"
  cat "$src" > "$dst" 2>/dev/null || rm -f "$dst"
}
show() { local f="$1"; [ -r "$f" ] || return 0; local v; v=$(cat "$f" 2>/dev/null) || return 0; echo "$(basename "$f") = $v"; copy "$f"; }

echo "# arctop keşif raporu — $(date -Iseconds)"
echo "kernel: $(uname -r)"
echo "perf_event_paranoid: $(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null)"

for card in /sys/class/drm/card[0-9]*; do
  [[ "$(basename "$card")" == *-* ]] && continue
  dev="$card/device"
  [ "$(cat "$dev/vendor" 2>/dev/null)" = "0x8086" ] || continue
  echo; echo "## $(basename "$card")"
  echo "device: $(cat "$dev/device")"
  echo "driver: $(basename "$(readlink -f "$dev/driver")")"
  echo "pci: $(basename "$(readlink -f "$dev")")"
  copy "$dev/vendor"; copy "$dev/device"
  if [ -n "$FIX" ]; then
    mkdir -p "$FIX$card/device"
    echo "$(basename "$(readlink -f "$dev/driver")")" > "$FIX$card/device/driver.name"
  fi

  echo "### frekans (i915)"
  for f in "$card"/gt_*freq_mhz; do show "$f"; done
  echo "### frekans (xe)"
  while read -r f; do [ -n "$f" ] && echo "${f#"$dev"/} = $(cat "$f" 2>/dev/null)" && copy "$f"; done \
    < <(find -L "$dev" -maxdepth 4 -path '*freq0*' -type f 2>/dev/null)

  echo "### hwmon"
  for h in "$dev"/hwmon/hwmon*; do
    [ -d "$h" ] || continue
    echo "-- $(basename "$h") name = $(cat "$h/name" 2>/dev/null)"; copy "$h/name"
    for f in "$h"/*_input "$h"/*_max "$h"/*_crit "$h"/*_label; do show "$f"; done
  done

  echo "### vram / tile ile ilgili girdiler"
  find -L "$dev" -maxdepth 3 \( -iname '*vram*' -o -name 'tile*' -o -iname '*lmem*' \) 2>/dev/null | head -40
done

echo; echo "## PMU"
found=0
for p in /sys/bus/event_source/devices/i915* /sys/bus/event_source/devices/xe*; do
  [ -d "$p" ] || continue; found=1
  echo "$(basename "$p") type=$(cat "$p/type")"
  ls "$p/events" 2>/dev/null | head -60 | sed 's/^/  /'
done
[ $found = 1 ] || echo "(i915/xe PMU bulunamadı)"

echo; echo "## fdinfo (okunabilen DRM istemcileri, ilk 5)"
n=0
for fd in /proc/[0-9]*/fdinfo/*; do
  grep -qs '^drm-driver:[[:space:]]*\(i915\|xe\)' "$fd" || continue
  pid=$(cut -d/ -f3 <<<"$fd")
  echo "### pid $pid ($(cat /proc/"$pid"/comm 2>/dev/null)) $fd"
  grep '^drm-' "$fd"
  if [ -n "$FIX" ]; then mkdir -p "$FIX$(dirname "$fd")"; grep '^drm-' "$fd" > "$FIX$fd"; cp /proc/"$pid"/comm "$FIX/proc/$pid/comm" 2>/dev/null; fi
  n=$((n+1)); [ $n -ge 5 ] && break
done
[ $n -gt 0 ] || echo "(hiç DRM istemcisi okunamadı — bir GPU uygulaması açıkken tekrar dene)"
[ -n "$FIX" ] && echo "# fixture yazıldı: $FIX" >&2
exit 0
