# ojuny-welcome

welcome menu for ojuny with initial setup flow

## Status

The first implementation provides the fixed first-boot page list, a GTK wizard
shell, completion state, and a next-steps window. It links the nix-settings-gui
engine and widget library.

## Pages

- Monitor setup
- Audio setup
- Browser select
- Default layout
- Starter styling
- Theme, font, and icons
- Hardware profile
- Quick applications
- Nixpkgs channel and tool sources
- Basic Nix configuration
- AppImage and Windows compatibility
- Default patches

## State

State is stored in `~/.config/ojuny-welcome/config.json`. The wizard marks
completion after a successful save. `--autostart` exits immediately when setup
is complete. `--reset` clears completion and starts the flow again.

The final rebuild action reads `~/.config/nix-settings-gui/config.json` first,
so it uses the same `terminal_command` and `rebuild_command` as nix-settings-gui.

## Build

From this directory:

```sh
cargo build
```

The local Cargo dependency points at `../nix-settings-gui`. The flake uses that
repository as a flake input and assembles both sources for the Rust build.

## Releases

A `v*` tag, a manual workflow run, or a commit on `main` whose title starts with
a version publishes `ojuny-welcome.bin`, `ojuny-welcome-flake.tar.gz`, and
`SHA256SUMS`. The release workflow records the asset in `binary-url.json`, so
the flake can install the prebuilt binary with `lib.mkBinary`.
