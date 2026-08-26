![PixelKit precision tools for Linux](docs/media-thumbnail-5to1.png)

# PixelKit

[![Latest release](https://img.shields.io/github/v/release/Kuucheen/PixelKit?style=flat&logo=github&label=release)](https://github.com/Kuucheen/PixelKit/releases/latest)
[![CI status](https://img.shields.io/github/actions/workflow/status/Kuucheen/PixelKit/ci.yml?branch=main&style=flat&label=CI&logo=github)](https://github.com/Kuucheen/PixelKit/actions/workflows/ci.yml?query=branch%3Amain)
[![GitHub stars](https://img.shields.io/github/stars/Kuucheen/PixelKit?style=flat&logo=github)](https://github.com/Kuucheen/PixelKit/stargazers)
[![MIT license](https://img.shields.io/github/license/Kuucheen/PixelKit?style=flat)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/Rust-1.88%2B-000000?style=flat&logo=rust&logoColor=white)](Cargo.toml)
[![Linux: X11 + Wayland](https://img.shields.io/badge/Linux-X11%20%2B%20Wayland-FCC624?style=flat&logo=linux&logoColor=black)](#x11-and-wayland-behavior)

PixelKit brings the Microsoft PowerToys Color Picker and Screen Ruler workflows
to Linux. It also includes a standalone magnifier and a QR and barcode scanner.
The whole application ships as one Rust binary and keeps its data on your
machine. It works on X11 and Wayland without privileged input or capture hooks.

> PixelKit is independent of Microsoft and PowerToys. This project uses the
> adjacent PowerToys source tree as an MIT-licensed behavioral reference.

## Demo

https://github.com/user-attachments/assets/fd06e8b0-96c1-4ea9-b514-56c151987239

## What PixelKit does

### Color picker, magnifier, and editor

The color picker is a fullscreen overlay that samples the original,
full-resolution capture. On a multi-monitor setup, each capture appears on the
monitor it came from. Large captures are split into lossless GPU-safe tiles
instead of being resized.

- The 13×13 loupe has tooltip and pointer-centered layouts, with up to 255 zoom
  levels controlled by the mouse wheel.
- The standalone magnifier has centered-grid and tooltip layouts. Its starting
  and maximum zoom, grid size, cursor, Wayland capture behavior, and shortcut
  are configurable.
- Arrow keys move one pixel. Hold Shift to move ten pixels at a time.
- Primary, middle, and secondary clicks can pick then edit, pick and close, or
  close without copying.
- Enter and Space pick a color. Escape and Backspace cancel. Escape also closes
  the editor opened after a pick.
- PixelKit keeps a de-duplicated color history; you choose how many entries it
  retains.
- The editor has RGB and HSV controls, three- or six-digit HEX input, four
  related color variations, and human-readable names based on the PowerToys
  bucket algorithm.
- History can be exported as JSON or text, grouped by color or format.
- Sixteen formats are built in: HEX, RGB, HSL, HSV, CMYK, HSB, HSI, HWB, NCol,
  CIE XYZ, CIE LAB, Oklab, Oklch, VEC4, Decimal, and HEX Int.
- Custom formats support PowerToys tokens such as `%ReX`, `%Gr`, `%Hu`, `%Sl`,
  `%Na`, and the full CIE and Oklab parameter set.

### QR and barcode scanner

The scanner checks the original full-resolution capture on a worker thread, so
the overlay stays responsive.

- Multiple codes in one image get numbered, clickable highlights.
- Outline and number-badge color and opacity are configurable.
- Supported formats include QR, Micro QR, Data Matrix, Aztec, PDF417, Code 39,
  Code 93, Code 128, EAN, UPC, ITF, Codabar, GS1 DataBar, Telepen, and MaxiCode.
- The result view shows the decoded format and complete text, with a button to
  copy it to the clipboard.
- HTTP and HTTPS results get an explicit `Open link` action. PixelKit shows the
  destination first, then hands it to the desktop portal.
- `Recapture` or `R` takes a new snapshot and scans it without including the
  PixelKit overlay.

### Screen ruler

The ruler measures rectangles and same-color spacing in cross, horizontal, and
vertical modes.

- Set a sum-of-RGB or per-channel edge tolerance from 0 to 255. The mouse wheel
  adjusts it in 15-point steps.
- Configure the crosshair color and end caps.
- Read measurements in pixels, inches, centimetres, or millimetres. PixelKit
  uses physical display data on X11 and a configurable fallback DPI on Wayland.
- Shift-click keeps several measurements on screen. Copying includes every
  retained measurement.
- Touch and pointer input are supported. Right-click cancels, `Ctrl+1` through
  `Ctrl+4` selects a tool, and snapshot recapture does not block the UI.
- During continuous capture on X11, PixelKit uses a transparent blank/capture
  cycle so its own lines do not appear in the sampled image.

### Linux integration

- Direct, low-latency X11 root capture and native key grabs.
- `xdg-desktop-portal` Screenshot and GlobalShortcuts interfaces on Wayland and
  inside Flatpak.
- A background shortcut daemon that does not poll the screen while idle.
- Desktop actions, AppStream metadata, scalable and high-resolution icons, man
  page, and systemd user unit.
- No telemetry, cloud service, root helper, or setuid binary. Decoded content
  stays local. Only a link you choose to open is handed to the desktop.

The shortcut daemon waits for events while idle. It does not initialize the GUI,
decoder, or screen capture until you launch a tool.

## X11 and Wayland behavior

| Capability | X11 | Wayland and Flatpak |
|---|---|---|
| Screen capture | Immediate direct capture | Screenshot portal. The compositor may ask for permission or a target |
| Picker, magnifier, scanner, and ruler overlays | One exact full-screen crop per monitor | One exact crop per monitor if XWayland geometry matches the portal image. Otherwise, one fullscreen portal snapshot |
| Global shortcuts | X11 key grab | GlobalShortcuts portal. The compositor chooses the final binding |
| Continuous ruler background | Live transparent recapture | Use `Recapture` or press `R` after the underlying content changes. The Screenshot portal is request-based |
| Clipboard | Native X11 selection | Native Wayland data control plus GUI clipboard fallback |

Wayland does not let regular applications silently capture other windows or
install global key grabs. The permission or target dialog is expected and does
not mean a package permission is missing. PixelKit uses the screenshot portal
for capture and XWayland only to position its overlays. If it cannot match the
portal image to the monitor geometry, it displays the snapshot in one fullscreen
overlay.

If your desktop does not provide the GlobalShortcuts portal, assign these
commands in the desktop's keyboard settings:

```text
pixelkit color-picker
pixelkit magnifier
pixelkit code-scanner
pixelkit screen-ruler
```

## Install from package repositories

Published x86_64 packages are available in the
[Fedora Copr repository](https://copr.fedorainfracloud.org/coprs/kuchen/pixelkit/)
and the [openSUSE Build Service project](https://build.opensuse.org/project/show/home:kuchen:PixelKit).
There is also an [AUR package](https://aur.archlinux.org/packages/pixelkit) for
Arch Linux.

### Fedora 44 and Rawhide

```bash
sudo dnf copr enable kuchen/pixelkit
sudo dnf install pixelkit
```

### Arch Linux with AUR

Install with an AUR helper such as `yay`:

```bash
yay -S pixelkit
```

Or build and install the package manually:

```bash
git clone https://aur.archlinux.org/pixelkit.git
cd pixelkit
makepkg -si
```

### Debian 13 and Ubuntu 24.04/26.04

Choose the `repo` value that matches your distribution, then add the OBS signing
key and APT source:

```bash
repo=xUbuntu_24.04 # Debian_13, xUbuntu_24.04, or xUbuntu_26.04
base="https://download.opensuse.org/repositories/home:/kuchen:/PixelKit/$repo"

sudo apt install curl gpg
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL "$base/Release.key" | gpg --dearmor | \
  sudo tee /etc/apt/keyrings/pixelkit-obs.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/pixelkit-obs.gpg] $base/ /" | \
  sudo tee /etc/apt/sources.list.d/pixelkit-obs.list
sudo apt update
sudo apt install pixelkit
```

The OBS signing-key fingerprint is
`C246 A272 BD2F B2A0 4044 ECDA C7C1 26FB 1D6D 878E`.

## Install from source

To build PixelKit, you need Rust 1.88 or newer, GCC, `make`, and the usual X11,
Wayland, and OpenGL runtime libraries. The Rust dependencies do not need GTK or
Qt development headers.

```bash
cargo test --all-targets --locked
cargo build --release --locked
sudo make install
```

For a user-only installation:

```bash
./scripts/install-user.sh
```

## Run PixelKit

Run `pixelkit` to open the main window. You can also launch each tool directly:

```bash
pixelkit color-picker
pixelkit magnifier
pixelkit color-editor
pixelkit code-scanner
pixelkit screen-ruler
```

On X11, the default shortcuts are `Super+Shift+C` for the picker,
`Super+Shift+Z` for the magnifier, `Super+Shift+Q` for the scanner, and
`Super+Shift+M` for the ruler. You can edit all four under
`Background shortcuts`. On Wayland, the compositor owns the final key
assignments.

Enable background shortcuts for native packages:

```bash
systemctl --user reenable pixelkit.service
systemctl --user restart pixelkit.service
```

The `reenable` command also removes obsolete early-start links left by older
PixelKit packages. This keeps the daemon from starting before the graphical
session is ready.

On Wayland, the desktop owns the active key combinations. If it registered the
actions without assigning keys, open its PixelKit shortcut page with:

```bash
pixelkit configure-shortcuts
```

## Building packages

The repository contains build definitions for the package formats listed below.
Release builds use the locked dependency graph. Distribution builds can use the
vendored source archive from `make dist`, so they do not need network access in
the build sandbox.

Use `scripts/build-packages.sh` as the main entry point. It refreshes Flatpak's
locked Cargo source hashes, runs the available checks once, reuses the release
binary where possible, and writes a verified `dist/SHA256SUMS`:

```bash
./scripts/build-packages.sh --list
./scripts/build-packages.sh
```

With no format argument, the script builds every format supported by the tools
installed on your machine. To build specific formats, name them explicitly, for
example `./scripts/build-packages.sh rpm deb`.

To start a new upstream release, run:

```bash
./scripts/build-packages.sh --clean --set-version 0.1.1
```

This updates Cargo, the lockfile, distribution package definitions, Flatpak,
Snap, Nix, AppStream, and the manual page. It also resets the RPM, Debian, and
Arch package revisions for the new version. To package another revision of the
same upstream version, use `--bump "Short changelog summary"` instead. Neither
option commits, tags, pushes, or publishes anything.

After the tagged GitHub release and its checksum manifest are public, publish
the matching vendored archive to the AUR with:

```bash
./scripts/publish-aur.sh
```

Add `--no-push` to prepare and verify the source without publishing it.

`make packages` calls the same script. Pass optional arguments through
`PACKAGE_ARGS`. If you have an RPM builder but not the development packages from
the spec, `--list` prints the missing package names. On Fedora, install all of
them with `sudo dnf builddep packaging/rpm/pixelkit.spec`.

| Ecosystem | Definition | Build command |
|---|---|---|
| Flatpak / Flathub | `packaging/flatpak/` | `./scripts/build-packages.sh flatpak` |
| Fedora / RHEL / DNF | `packaging/rpm/pixelkit.spec` | `./scripts/build-packages.sh rpm` |
| openSUSE / Zypper | `packaging/opensuse/pixelkit.spec` | `./scripts/build-packages.sh opensuse` |
| Debian / Ubuntu / APT | `debian/` | `./scripts/build-packages.sh deb-source` |
| Arch / pacman / AUR | `packaging/arch/PKGBUILD` | `./scripts/build-packages.sh arch` |
| Snap Store | `snap/snapcraft.yaml` | `./scripts/build-packages.sh snap` |
| Nix / NixOS | `flake.nix` | `./scripts/build-packages.sh nix` |
| AppImage | `packaging/appimage/build-appimage.sh` | `./scripts/build-packages.sh appimage` |

See the [packaging and release guide](docs/PACKAGING.md) for namespace ownership,
checksums, repository submission, and validation details.

## Configuration

PixelKit stores its settings and color history as readable, version-tolerant
JSON:

- `${XDG_CONFIG_HOME:-~/.config}/pixelkit/settings.json`
- `${XDG_DATA_HOME:-~/.local/share}/pixelkit/history.json`

Run `pixelkit paths` to print the exact locations. If the settings file is
corrupt, PixelKit leaves the history alone, reports the error in the UI, and
loads safe defaults.

## Development and tests

```bash
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo run -- formats '#ff0000'
cargo run -- color-picker --image test-screen.png
cargo run -- magnifier --image test-screen.png
cargo run -- code-scanner --image test-screen.png
cargo run -- screen-ruler --image test-screen.png
```

The `--image` option runs the same picker, magnifier, scanner, and ruler UI and
algorithms without requesting a desktop capture. Before a release, test on X11
and Wayland, especially with multiple monitors and mixed scale factors.

## License and attribution

PixelKit is licensed under MIT. [NOTICE](NOTICE) lists the PowerToys and barcode
decoder attribution. Third-party Rust crates keep their own licenses. Use
`cargo deny` and the distribution tools to produce the dependency license
inventory for a release.
