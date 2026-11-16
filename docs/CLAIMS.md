# Claims and proof

This is the evidence register for promises shown in the UI, Help, onboarding,
and [PRIVACY.md](../PRIVACY.md). A claim is release copy only when its proof is
listed here.

| Claim | Surface | Proof |
|---|---|---|
| Nothing leaves this Mac | Help, Privacy and Trust | **NEEDS-HUMAN:** run the live `lsof`/`nettop` measurement in [PRIVACY.md](../PRIVACY.md#measurement-record) during a voice command; the installed launchd job was absent during this session. |
| Pause everything stops microphone capture | Settings, Help, Privacy | Agent log contains `EVO voice: microphone capture paused`; `voice.status` is `paused`. |
| The true state is Listening, Paused, or Off | Home, tray, notch | The voice thread writes `voice.status`; bridge/Home/tray read it. |
| Closing the window keeps the menu bar available | Help and using guide | Manual macOS GUI check: close the window, then open the status item. |
| Quit Evo completely stops the agent and daemon | Tray and Settings | Bridge quit issues `launchctl bootout` and terminates the daemon when `keep_background` is false. |
| Evo brings its window forward | Help | `presence.rs` calls `activateIgnoringOtherApps:` and `makeKeyAndOrderFront:`. |
| Storage stays local and permissions are restricted | Privacy | Startup permission checks verify directory mode 700 and log/socket mode 600. |

Claims requiring a macOS GUI or process observer are manual checks and must not
be described as automated test results.