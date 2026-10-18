# Enabling always-on capture (survives reboots)

The voice-capable `com.evo.agent` is the preferred always-on owner: it keeps
the microphone, voice socket, and daemon child alive without the desktop app.
On machines without that agent, `install-agents.sh` installs the legacy
`dev.evo.capture` daemon LaunchAgent instead. Never run both against the same
storage root.

1. One-time, stable signing (so the grant survives every rebuild):
   ```sh
   bash scripts/sign-dev.sh --create-identity
   ```
   Follow the printed Keychain Access steps (a self-signed "Evo Code
   Signing" certificate), then:
   ```sh
   cargo build --release && bash scripts/sign-dev.sh
   ```

2. Grant Accessibility to the daemon:
   ```sh
    bash scripts/install-agents.sh   # starts ledger and the safe daemon owner
   ```
   The Evo desktop will honestly show "PARTIAL — Accessibility permission
   required". Click **Open System Settings** → Privacy & Security →
   Accessibility → allow `evo-daemon` (from `target/release/`).

3. Restart capture; launchd picks it up:
   ```sh
    launchctl kickstart -k gui/$(id -u)/com.evo.agent
   ```

Remove always-on capture any time:
```sh
bash scripts/install-agents.sh --remove
```

The ledger engine needs no permissions and already runs under launchd.
