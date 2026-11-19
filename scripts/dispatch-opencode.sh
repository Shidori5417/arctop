#!/usr/bin/env bash
# Görev kartını OpenCode (GLM 5.3) ile ayrı bir worktree'de çalıştırır.
# Kullanım: scripts/dispatch-opencode.sh .tasks/T-004.md
# Model ID'si sağlayıcıya göre değişir: `opencode models` ile doğru ID'yi bul ve OPENCODE_MODEL ile ver.
set -euo pipefail
CARD="$(realpath "${1:?kullanım: dispatch-opencode.sh .tasks/T-xxx.md}")"
MODEL="${OPENCODE_MODEL:-zai-coding-plan/glm-5.3}"
ID="$(basename "$CARD" .md)"
ROOT="$(git rev-parse --show-toplevel)"
NAME="$(basename "$ROOT")"
WT="$ROOT/../$NAME-$ID"

[ -d "$WT" ] || git -C "$ROOT" worktree add "$WT" -b "task/$ID"
mkdir -p "$WT/.tasks/done" "$ROOT/.tasks/logs"
cp "$CARD" "$WT/.tasks/"

cd "$WT"
opencode run --model "$MODEL" \
  "AGENTS.md dosyasını ve .tasks/$ID.md görev kartını oku, karta harfiyen uy. Bitince doğrulama komutlarını çalıştır, .tasks/done/$ID.report.md raporunu yaz ve commit et." \
  > "$ROOT/.tasks/logs/$ID.log" 2>&1 && STATUS=ok || STATUS=hata

echo "[$STATUS] $ID — rapor: $WT/.tasks/done/$ID.report.md — log: .tasks/logs/$ID.log"
