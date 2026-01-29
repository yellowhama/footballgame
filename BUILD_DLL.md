# Rebuild the Godot GDExtension DLL

This project ships a Rust GDExtension in `godot_extension/`. Rebuild the DLL and deploy it to Godot's expected path.

## Prerequisites
- Rust toolchain (stable)
- Cargo in PATH
- Close the Godot editor (Windows will lock the DLL while open)

## One‑liner (Windows PowerShell)

```
# From project root
pwsh -File tools/build_gdext.ps1 -Release
```

This will:
- Run `cargo build -p football_rust --release`
- Copy the newest built `football_rust.dll` → `bin/football_rust.dll`

Godot loads the DLL from `res://bin/football_rust.dll` (see `godot_bridge.gdextension`).

## Manual steps (if you prefer)

1) Build
```
cargo build -p football_rust --release
```

2) Copy the DLL
```
copy .\target\release\football_rust.dll .\bin\football_rust.dll
```

3) Restart Godot
- Verify in the console that the build timestamp changes and that `start_simulation` is available.
- You should no longer see "Fallback async start" logs; the native async path will be used.

## Verify
- On match start, OpenFootballAPI awaits `rust_engine.simulation_completed` without using the fallback thread.
- Loading overlay closes on completion.

If you still see errors after rebuild, paste the exact log lines and we’ll trace them; but with the async DLL, the wrapper’s fallback becomes unnecessary and stability improves.
