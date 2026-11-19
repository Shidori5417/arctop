#!/usr/bin/env bash
# Görev kartını Antigravity (Hydrogen) için hazırlar: worktree açar ve yapıştırılacak prompt'u verir.
# Kullanım: scripts/handoff-antigravity.sh T-009
set -euo pipefail
ID="${1:?kullanım: handoff-antigravity.sh T-xxx}"
ROOT="$(git rev-parse --show-toplevel)"
NAME="$(basename "$ROOT")"
CARD="$ROOT/.tasks/$ID.md"
[ -f "$CARD" ] || { echo "kart yok: $CARD" >&2; exit 1; }
WT="$ROOT/../$NAME-$ID"

[ -d "$WT" ] || git -C "$ROOT" worktree add "$WT" -b "task/$ID"
mkdir -p "$WT/.tasks/done"
cp "$CARD" "$WT/.tasks/"

PROMPT="AGENTS.md dosyasını ve .tasks/$ID.md görev kartını oku, karta harfiyen uy. Yalnızca kartta listelenen dosyaları değiştir. Bitince doğrulama komutlarını çalıştır, .tasks/done/$ID.report.md raporunu yaz ve commit et."

echo "1) Antigravity'de şu klasörü aç: $WT"
echo "2) Modeli seç: Hydrogen"
echo "3) Şu prompt'u yapıştır:"
echo
echo "$PROMPT"
if command -v wl-copy >/dev/null 2>&1; then printf '%s' "$PROMPT" | wl-copy && echo && echo "(prompt panoya kopyalandı)"; fi
