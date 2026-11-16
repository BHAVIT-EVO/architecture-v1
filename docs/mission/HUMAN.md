# What needs you

Only items a program cannot do. Each has the exact steps.

## 1. Microphone and Accessibility toggles
System Settings → Privacy & Security.
1. **Microphone**: switch Evo on (and `evo-agent` if it is listed separately).
2. **Accessibility**: switch Evo on. If an old Evo row is already on but the
   permission does not work, remove the row with the minus button and add
   `/Applications/Evo.app` again — an ad-hoc signature changes the code
   requirement on every build.
3. **Login Items & Extensions**: look for `com.evo.agent` / `evo-agent`. macOS
   recorded it as disabled (`sfltool dumpbtm`: `Disposition: [disabled, …]`).
   Switch it on. Code can `launchctl enable` and bootstrap, but macOS refuses to
   re-enable a background item you switched off.

## 2. Duplicate-bundle check (Phase 0.5)
The unscoped `mdfind "kMDItemCFBundleIdentifier == 'dev.evo.desktop'"` is
refused by this runner, and Spotlight returned nothing even for `/Applications`.
Run it yourself in Terminal and confirm only `/Applications/Evo.app` appears.

## 3. The hardware notch, by eye
Open Evo on the notched Mac and check: the black overlay lines up with the
hardware notch, no seam in either theme, the overlay stays pure black in Day.

## 4. A real voice test
Launch Evo, say “Hey Evo, open Muse” three times in a quiet room. The notch
should wake on the first syllable, show what it heard, then open the work.

## 5. Real Chrome and Finder arrangement
Grant Accessibility, then Continue a work that had three or more windows. Check
that the windows sit inside the visible frame, never cover the Dock or menu bar,
and that “Put things back” returns them.

## 6. Menu bar icon in light and dark
Switch macOS appearance and confirm the Evo menu-bar item is legible in both.

## 7. Launch at login
Log out and back in with “Keep Evo listening in the background” on, and confirm
the agent is running before the window is opened.

## 8. A visual pass of Day and Dusk
Flip Appearance → Day, Dusk, Auto and walk Home, Search, Settings, Activity,
Permissions, Diagnostics, About, onboarding.
