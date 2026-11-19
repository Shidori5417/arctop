# arctop

Intel Arc GPU'lar için terminal tabanlı canlı izleme aracı. Rust ve ratatui ile yazılır. nvtop'un Arc tarafında eksik kalan kısımlarını tamamlamak için tasarlandı.

- Frekans, güç (W), sıcaklık, fan, voltaj
- Motor doluluğu (render / copy / video / video-enhance / compute)
- Süreç bazlı GPU ve VRAM kullanımı (fdinfo)
- Canlı grafikler (60 sn ve 5 dk geçmiş)
- Root gerektirmez. Okunamayan metrikler "—" olarak gösterilir ve nedeni yardım ekranında açıklanır
- `--json` / `--once` ile scriptlerden kullanılabilir

**Birincil hedef:** Arc A750 (DG2, `i915` sürücüsü). **İkincil hedef:** `xe` sürücüsü ve Battlemage.
**Durum:** Faz 1 tamam, sırada Faz 2 (metrik kaynakları) — bkz. `docs/ROADMAP.md`

## Ajanlı geliştirme düzeni

| Rol | Model | Araç |
|---|---|---|
| Orkestratör (kod yazmaz) | Claude Opus | Claude Code |
| Çekirdek kod | Claude Sonnet | Claude Code alt ajanı `rust-coder` |
| Yardımcı / log özeti | Claude Haiku | Claude Code alt ajanı `helper` |
| Bağımsız modüller | GLM 5.3 | OpenCode (`scripts/dispatch-opencode.sh`) |
| TUI ekranları / ikinci göz | Hydrogen | Antigravity (`scripts/handoff-antigravity.sh`) |

Kurallar `CLAUDE.md` (orkestratör) ve `AGENTS.md` (uygulayıcılar) dosyalarındadır.

### Başlangıç
```bash
cd arctop
git init && git add -A && git commit -m "init: planlama dokümanları"
# GLM model ID'ni kontrol et ve gerekirse ayarla:
opencode models | grep -i glm
export OPENCODE_MODEL=<sağlayıcı/glm-5.3>

claude --model opus
> docs/ROADMAP.md Faz 0'dan başla.
```

**Faz 0 senden bir adım ister:** Kartı bir oyun ya da `glxgears` açıkken çalıştır, böylece fdinfo verisi de görünür:
```bash
scripts/discover-gpu.sh > docs/hw/a750.txt
```

## Lisans
TBD
