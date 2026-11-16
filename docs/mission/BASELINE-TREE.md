# Baseline working tree (Phase 0.1)

223 paths were dirty when the mission started. None were models, secrets or
files over 5 MB (see evidence/0-inventory.txt). The committed set is source and
docs only; the excluded set is listed at the bottom and in DECISIONS.md D-001.

| status | bytes | path |
| --- | --- | --- |
| `??` | .github/ | `0` |
| ` D` | .kiro/settings/mcp.json | `-1` |
| `??` | ARCHITECTURE.md | `3083` |
| `??` | CHANGELOG.md | `2503` |
| `??` | CONTRIBUTING.md | `2322` |
| ` M` | Cargo.lock | `162210` |
| ` D` | DEMO.md | `-1` |
| ` M` | DESIGN.md | `15237` |
| `??` | EXPERIENCE.md | `4965` |
| `??` | GLOSSARY.md | `5170` |
| `??` | PRIVACY.md | `6378` |
| `??` | PROJECT_BRIEF.md | `13547` |
| `??` | QUESTIONS.md | `8558` |
| ` M` | README.md | `22955` |
| `??` | SECURITY.md | `7431` |
| `??` | SPEC.md | `11806` |
| `??` | TODO.md | `4400` |
| `??` | assets/ | `0` |
| ` M` | crates/evo-artifact/Cargo.toml | `191` |
| ` M` | crates/evo-capture/Cargo.toml | `190` |
| ` M` | crates/evo-capture/examples/macos_activation_demo.rs | `2143` |
| ` M` | crates/evo-capture/examples/probe_delivery.rs | `1590` |
| ` M` | crates/evo-capture/examples/probe_fsevents.rs | `3265` |
| ` M` | crates/evo-capture/examples/probe_sources.rs | `2112` |
| ` M` | crates/evo-capture/examples/probe_url.rs | `9201` |
| ` M` | crates/evo-capture/src/adapters/macos.rs | `34974` |
| ` M` | crates/evo-daemon/Cargo.toml | `1060` |
| ` M` | crates/evo-daemon/examples/debug_storage.rs | `1280` |
| ` M` | crates/evo-daemon/examples/measure_fixture_retrieval.rs | `19748` |
| ` M` | crates/evo-daemon/examples/measure_vague_retrieval.rs | `13351` |
| ` M` | crates/evo-daemon/examples/probe_runtime.rs | `1568` |
| ` M` | crates/evo-daemon/examples/sqlite_probe.rs | `1593` |
| ` M` | crates/evo-daemon/src/classification.rs | `20927` |
| ` M` | crates/evo-daemon/src/classification_rules.rs | `31574` |
| ` M` | crates/evo-daemon/src/feedback.rs | `12849` |
| ` M` | crates/evo-daemon/src/lib.rs | `3049` |
| ` M` | crates/evo-daemon/src/main.rs | `2540` |
| ` M` | crates/evo-daemon/src/name_index.rs | `20770` |
| ` M` | crates/evo-daemon/src/threads.rs | `64321` |
| ` M` | crates/evo-daemon/src/voice.rs | `126919` |
| ` M` | crates/evo-daemon/src/work_understanding.rs | `43164` |
| `??` | crates/evo-desktop/ADR-0001-clearing-visual-system.md | `1740` |
| ` M` | crates/evo-desktop/Cargo.toml | `2131` |
| `??` | crates/evo-desktop/SPEC.md | `4036` |
| `??` | crates/evo-desktop/TODO.md | `689` |
| `??` | crates/evo-desktop/assets/fonts/LICENSE-PixelifySans.txt | `4488` |
| `??` | crates/evo-desktop/assets/fonts/PixelifySans.ttf | `79160` |
| ` M` | crates/evo-desktop/build.rs | `1027` |
| `??` | crates/evo-desktop/examples/bridge_cli.rs | `1074` |
| ` M` | crates/evo-desktop/examples/qa_capture.rs | `31827` |
| ` M` | crates/evo-desktop/examples/qa_layout.rs | `29288` |
| `??` | crates/evo-desktop/resources/ | `0` |
| ` M` | crates/evo-desktop/src/app.rs | `114948` |
| ` M` | crates/evo-desktop/src/bin/evo-agent-install.rs | `19610` |
| ` M` | crates/evo-desktop/src/bin/evo-agent.rs | `23832` |
| `??` | crates/evo-desktop/src/bridge.rs | `71734` |
| `??` | crates/evo-desktop/src/card/ | `0` |
| `??` | crates/evo-desktop/src/collage.rs | `1261` |
| `??` | crates/evo-desktop/src/copy.rs | `4616` |
| ` M` | crates/evo-desktop/src/detail.rs | `132112` |
| ` M` | crates/evo-desktop/src/dock.rs | `9763` |
| ` D` | crates/evo-desktop/src/home.rs | `-1` |
| `??` | crates/evo-desktop/src/home/ | `0` |
| ` M` | crates/evo-desktop/src/lib.rs | `2434` |
| ` M` | crates/evo-desktop/src/main.rs | `4230` |
| ` M` | crates/evo-desktop/src/notch.rs | `58240` |
| ` M` | crates/evo-desktop/src/notch_window.rs | `316589` |
| `??` | crates/evo-desktop/src/observability.rs | `12472` |
| ` M` | crates/evo-desktop/src/pod_surface.rs | `45794` |
| ` M` | crates/evo-desktop/src/presence.rs | `33761` |
| ` M` | crates/evo-desktop/src/rack.rs | `50090` |
| ` M` | crates/evo-desktop/src/rooms.rs | `10347` |
| ` M` | crates/evo-desktop/src/scoped_restore.rs | `49777` |
| `??` | crates/evo-desktop/src/security/ | `0` |
| ` M` | crates/evo-desktop/src/shell.rs | `22549` |
| `??` | crates/evo-desktop/src/stage/ | `0` |
| ` M` | crates/evo-desktop/src/theme.rs | `47679` |
| ` M` | crates/evo-desktop/src/ui.rs | `42754` |
| `??` | crates/evo-desktop/src/ui_state.rs | `12216` |
| ` M` | crates/evo-desktop/src/unified_workspace.rs | `6953` |
| ` M` | crates/evo-desktop/src/voice.rs | `62255` |
| `??` | crates/evo-desktop/src/webshell.rs | `40593` |
| ` M` | crates/evo-desktop/src/websurface.rs | `30545` |
| `??` | crates/evo-desktop/tests/journeys.rs | `6379` |
| `??` | crates/evo-desktop/ui/ | `0` |
| ` M` | crates/evo-engagement/Cargo.toml | `151` |
| ` M` | crates/evo-engagement/src/bin/metamorphic_evaluation.rs | `16011` |
| ` M` | crates/evo-execution/Cargo.toml | `427` |
| ` M` | crates/evo-execution/src/bin/room_trace.rs | `1760` |
| ` M` | crates/evo-execution/src/browser_macos.rs | `69471` |
| ` M` | crates/evo-execution/src/lib.rs | `4080` |
| ` M` | crates/evo-execution/src/macos.rs | `82053` |
| ` M` | crates/evo-execution/src/room.rs | `34235` |
| ` M` | crates/evo-execution/src/room_macos.rs | `20300` |
| ` M` | crates/evo-ledger/Cargo.toml | `883` |
| ` M` | crates/evo-ledger/src/hypotheses.rs | `69935` |
| ` M` | crates/evo-ledger/src/lib.rs | `18956` |
| ` M` | crates/evo-ledger/src/thread_gate.rs | `28947` |
| ` M` | crates/evo-ledger/tests/thread_gate.rs | `3555` |
| ` M` | crates/evo-observation/Cargo.toml | `154` |
| ` M` | crates/evo-observation/src/validation.rs | `34944` |
| ` M` | crates/evo-pods/Cargo.toml | `373` |
| ` M` | crates/evo-pods/src/layout.rs | `32342` |
| ` M` | crates/evo-pods/src/lib.rs | `1905` |
| ` M` | crates/evo-pods/tests/layout.rs | `23864` |
| ` M` | crates/evo-restoration/Cargo.toml | `242` |
| ` M` | crates/evo-retrieval/Cargo.toml | `262` |
| ` M` | crates/evo-semantic/Cargo.toml | `227` |
| ` M` | crates/evo-semantic/src/lib.rs | `37479` |
| ` M` | crates/evo-storage/Cargo.toml | `82` |
| ` M` | crates/evo-storage/src/lib.rs | `1564` |
| ` M` | crates/evo-storage/src/root.rs | `23400` |
| ` M` | crates/evo-threads/Cargo.toml | `342` |
| ` M` | crates/evo-threads/src/assign.rs | `48026` |
| ` D` | crates/evo-threads/src/bin/boundary_dump.rs | `-1` |
| ` M` | crates/evo-threads/src/brief.rs | `4813` |
| ` M` | crates/evo-threads/src/retrieval.rs | `26883` |
| ` M` | crates/evo-threads/tests/classification.rs | `5115` |
| ` M` | crates/evo-voice/Cargo.toml | `1409` |
| ` M` | crates/evo-voice/src/audio.rs | `14068` |
| `??` | crates/evo-voice/src/bin/replay_wav.rs | `2796` |
| ` M` | crates/evo-voice/src/driver.rs | `87130` |
| ` M` | crates/evo-voice/src/transcribe.rs | `10128` |
| ` M` | crates/evo-voice/src/wake.rs | `16660` |
| ` M` | crates/evo-voice/src/wake_model.rs | `21789` |
| ` M` | crates/evo-workspace/Cargo.toml | `348` |
| ` M` | crates/evo-workspace/src/workspace_id.rs | `4249` |
| `??` | deny.toml | `796` |
| `??` | design/ | `0` |
| `??` | docs/ANALYTICS.md | `2354` |
| `??` | docs/AUDIT_RULES.md | `2559` |
| `??` | docs/BRAND.md | `944` |
| `??` | docs/BRAND_AUDIT.md | `636` |
| `??` | docs/BRIDGE.md | `5558` |
| `??` | docs/CLAIMS.md | `1479` |
| `??` | docs/DESIGN_LAW.md | `5898` |
| `??` | docs/E2E_REPORT.md | `5629` |
| ` D` | docs/ENGINEERING.md | `-1` |
| `??` | docs/FIX_RULES.md | `1649` |
| `??` | docs/INTEGRATION.md | `12272` |
| `??` | docs/LAUNCH.md | `5946` |
| ` M` | docs/RELEASE-CHECKLIST.md | `19311` |
| `??` | docs/RELIABILITY.md | `4229` |
| `??` | docs/UNSAFE.md | `2226` |
| `??` | docs/adr/ | `0` |
| `??` | docs/audit/AUDIT-1.md | `6084` |
| `??` | docs/audit/AUDIT-2.md | `2477` |
| `??` | docs/audit/AUDIT-3.md | `1622` |
| `??` | docs/audit/AUDIT-4.md | `1474` |
| `??` | docs/audit/F2.md | `1082` |
| `??` | docs/audit/ZVARIANT.md | `193` |
| `??` | docs/design/mockup-home-v3.html | `20749` |
| `??` | docs/fix/ | `0` |
| `??` | docs/mission/ | `0` |
| `??` | docs/shots/ | `0` |
| `??` | docs/social-preview.png | `27184` |
| `??` | docs/social-preview.svg | `238` |
| ` D` | exports/ship-happens/Evo_Ship_Happens_Build_Plan.pdf | `-1` |
| ` D` | exports/ship-happens/Evo_Stage_Card.pdf | `-1` |
| ` D` | exports/ship-happens/JURISDICTION.md | `-1` |
| ` D` | exports/ship-happens/README.md | `-1` |
| ` D` | exports/ship-happens/RUNBOOK.md | `-1` |
| ` D` | exports/ship-happens/SUBMISSION.md | `-1` |
| ` D` | exports/ship-happens/ask.py | `-1` |
| ` D` | exports/ship-happens/boundary.html | `-1` |
| ` D` | exports/ship-happens/boundary.json | `-1` |
| ` D` | exports/ship-happens/evo_server.py | `-1` |
| ` D` | exports/ship-happens/evo_ui.html | `-1` |
| ` D` | exports/ship-happens/mcp_probe.py | `-1` |
| ` D` | exports/ship-happens/sarvam_check.py | `-1` |
| ` D` | exports/ship-happens/serve.py | `-1` |
| `??` | graphify-out/ | `0` |
| `??` | screenshot.png | `822064` |
| ` M` | scripts/build-app.sh | `23541` |
| `??` | scripts/check-ui-bundle.sh | `1256` |
| `??` | scripts/selftest.sh | `3809` |
| `??` | scripts/shot-panel.sh | `926` |
| `??` | scripts/theme-proof.mjs | `3785` |
| `??` | site/404.html | `2856` |
| ` D` | site/DEPLOY.md | `-1` |
| ` D` | site/README-demo.md | `-1` |
| ` D` | site/README.md | `-1` |
| `??` | site/about.html | `3669` |
| `??` | site/assets/ | `0` |
| `??` | site/changelog.html | `3145` |
| `??` | site/contact.html | `3614` |
| ` D` | site/demo.html | `-1` |
| `??` | site/download.html | `3980` |
| ` D` | site/film.html | `-1` |
| ` D` | site/fonts/mono-500.woff2 | `-1` |
| ` D` | site/fonts/mono.woff2 | `-1` |
| ` D` | site/fonts/sans-500.woff2 | `-1` |
| ` D` | site/fonts/sans-italic.woff2 | `-1` |
| ` D` | site/fonts/sans.woff2 | `-1` |
| ` D` | site/fonts/serif-italic.woff2 | `-1` |
| ` D` | site/fonts/serif.woff2 | `-1` |
| `??` | site/guide.html | `5381` |
| ` M` | site/index.html | `9848` |
| `??` | site/privacy-and-trust.html | `4296` |
| `??` | site/privacy.html | `4551` |
| `??` | site/robots.txt | `107` |
| `??` | site/site.css | `9300` |
| `??` | site/site.js | `2210` |
| `??` | site/site.webmanifest | `411` |
| `??` | site/sitemap.xml | `798` |
| ` D` | site/source.html | `-1` |
| ` D` | site/src/00-head.html | `-1` |
| ` D` | site/src/10-body.html | `-1` |
| ` D` | site/src/20-js.html | `-1` |
| ` D` | site/src/console-body.html | `-1` |
| ` D` | site/src/console-js.html | `-1` |
| ` D` | site/src/source-body.html | `-1` |
| `??` | site/terms.html | `3472` |
| `??` | site/thank-you.html | `3722` |
| `??` | tests/ | `0` |
| ` D` | tools/build_console.py | `-1` |
| ` D` | tools/build_plan.py | `-1` |
| ` D` | tools/build_site.py | `-1` |
| ` D` | tools/build_site_v2.py | `-1` |
| ` D` | tools/build_source.py | `-1` |
| ` D` | tools/deploy_site.sh | `-1` |
| ` D` | tools/stage_card.py | `-1` |
| `??` | ui/ | `0` |

## Left uncommitted

- `graphify-out/` — generated code graph, now gitignored
- `.kiro/settings/mcp.json` — editor settings
- `exports/`, `site/`, `tools/` — unrelated site work in progress
- `screenshot.png` — root capture

Full diff: `~/Evo-backups/baseline-tree.patch`; untracked list:
`~/Evo-backups/baseline-untracked.txt`.
