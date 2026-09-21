# Enabling always-on capture (survives reboots)

Capture currently runs while the Evo desktop app is open (the desktop
spawns the daemon). To make capture survive reboots — launchd owns it —
there is a **one-time Accessibility grant** to give:

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
   bash scripts/install-agents.sh   # starts capture under launchd
   ```
   The Evo desktop will honestly show "PARTIAL — Accessibility permission
   required". Click **Open System Settings** → Privacy & Security →
   Accessibility → allow `evo-daemon` (from `target/release/`).

3. Restart capture; launchd picks it up:
   ```sh
   launchctl kickstart -k gui/$(id -u)/dev.evo.capture
   ```

Remove always-on capture any time:
```sh
bash scripts/install-agents.sh --remove
```

The ledger engine needs no permissions and already runs under launchd.
