# Cross-Platform and Distribution

## Cross-Platform Status

Happy Wakey is cross-platform by architecture, but macOS is the only platform verified in the current development pass.

The source does not rely on a macOS-only UI framework. Qt abstracts windows,
controls, graphics, input, and accessibility across platforms; btleplug maps to
CoreBluetooth, WinRT, and BlueZ. Rust service and persistence code is portable.

## Build Model

Build one release on each target operating system so native UI, BLE permissions,
and packaging can be verified on the real target.

| Target | Compiler/runtime | Qt deployment tool | Typical package |
| --- | --- | --- | --- |
| macOS | Apple Clang through Rust toolchain | `macdeployqt` | signed/notarized `.app` in `.dmg` |
| Windows | Rust MSVC target + matching Qt MSVC build | `windeployqt` | signed MSI or installer EXE |
| Linux | Native Rust/GCC/Clang + distro Qt | `linuxdeployqt` or Flatpak tooling | Flatpak and/or AppImage; optional `.deb`/`.rpm` |

## Shared Build Steps

1. Install a supported Rust toolchain.
2. Install Qt 6 with Quick and Controls for the target architecture.
3. Make Qt discoverable to CMake/CXX-Qt (`PATH`, `CMAKE_PREFIX_PATH`, or platform tooling).
4. Run `cargo test --locked`.
5. Run `cargo build --release --locked`.
6. Create the platform application/package structure.
7. Copy/deploy Qt libraries, QML modules, plugins, and locales.
8. Sign the complete package.
9. Run the installed artifact, not only the build-tree executable.
10. Publish checksums and release metadata.

## macOS

### Package Shape

```text
Happy Wakey.app/
  Contents/
    Info.plist
    MacOS/happy-wakey
    Frameworks/
    PlugIns/
    Resources/
```

Recommended flow:

1. Build a universal binary only if all Rust and Qt dependencies are available for both `arm64` and `x86_64`; otherwise publish architecture-specific builds.
2. Create `Info.plist` with a stable bundle identifier such as `com.happywakey.app`.
3. Merge `deploy/macos/Info.plist` into the bundle metadata and run `macdeployqt`.
4. Sign nested helpers/frameworks first and the outer application last with hardened runtime.
5. Submit to Apple notarization and staple the result.
6. Put the notarized app in a signed or notarized DMG.

Release acceptance should verify OAuth loopback login, Bluetooth permission and
discovery, network access, config permissions, native notification delivery,
and app relaunch from `/Applications`. The bundle identifier must match
`HAPPY_WAKEY_BUNDLE_ID`.

## Windows

Use the Rust MSVC target and a Qt build compiled for the same MSVC version and
architecture. `windeployqt` should collect DLLs, platform plugins, and QML
modules; acceptance must exercise the WinRT Bluetooth adapter.

The installer should:

- install under Program Files;
- create Start Menu entries;
- register uninstall metadata;
- preserve user config during upgrade;
- optionally configure launch at login;
- install the Visual C++ runtime when required;
- use Authenticode signing.

MSI via WiX is a strong default for managed environments. An installer EXE can provide a simpler consumer setup.

## Linux

Flatpak is the cleanest first production target because it defines a runtime
and portal/BlueZ permissions. AppImage is useful as a direct-download option.

Test at least:

- Ubuntu LTS under Wayland and X11;
- Fedora under Wayland;
- a representative KDE environment;
- BlueZ/DBus discovery and permission behavior;
- desktop portal URL opening and notifications;
- font and high-DPI behavior.

## CI Matrix

A release workflow should use native runners:

```yaml
strategy:
  matrix:
    include:
      - os: macos-14
        target: aarch64-apple-darwin
      - os: macos-13
        target: x86_64-apple-darwin
      - os: windows-2022
        target: x86_64-pc-windows-msvc
      - os: ubuntu-24.04
        target: x86_64-unknown-linux-gnu
```

Each job should cache Cargo artifacts and Qt separately, run tests, build release mode, deploy Qt, launch an installed-package smoke test, and upload a signed artifact. Signing and notarization should occur only for protected release tags.

## OAuth in Packaged Apps

The current desktop OAuth flow uses PKCE and a fixed loopback callback:

`http://127.0.0.1:47217/callback`

That URL must be allow-listed in Supabase. Google and Microsoft still redirect through Supabase's provider callback before returning to the loopback URL.

A fixed port is simple but can conflict with another process. A production design should either reserve/document the port carefully or adopt an operating-system URL scheme such as `happywakey://oauth/callback`, with platform-specific protocol registration and Supabase allow-listing.

## Updates

No updater is implemented. A safe update system needs:

- signed release metadata;
- HTTPS artifact delivery;
- signature/hash verification before install;
- rollback or failure recovery;
- staged channels such as stable and beta;
- platform-specific elevation behavior;
- schema/config backward compatibility.

Do not add an updater that executes unsigned downloads. Until a signed updater exists, publish signed packages and let users install releases explicitly.

## Distribution Readiness Checklist

- [ ] Stable product name, bundle ID, icons, versioning, and license metadata
- [ ] Release-mode tests on macOS, Windows, and Linux
- [ ] Qt and native Bluetooth deployment verified outside the build tree
- [ ] macOS signing, hardened runtime, notarization, and DMG
- [ ] Windows Authenticode signing and installer
- [ ] Linux Flatpak/AppImage permissions and sandbox validation
- [ ] OAuth provider production credentials and redirect configuration
- [ ] OS credential-vault storage for tokens
- [x] Running-app reminder scheduler and native notification delivery on macOS
- [ ] Notification permission UX, actions/snooze, and Windows/Linux installed-package verification
- [ ] Privacy policy and external-provider attribution
- [ ] Crash reporting and update strategy decisions
- [ ] Reproducible CI release pipeline
