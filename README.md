## CHMER 5

![CHMER Logo](../chmer.png)

**CHMER 5** (Chess Machine Engine Runtime v5) is a standalone programming language for chess development.

- **User language**: CHMER (`.ch`)
- **Implementation**: Rust (compiler + VM + runtime), hidden from users
- **Binary**: `chmer`
- **License**: LGPL-3.0-or-later (see `LICENSE`)

## Quick start (precompiled)

- **Linux/macOS/*BSD (sh installer)**:

```bash
curl -fsSL <RELEASE_BASE_URL>/install.sh | sh
```

- **Windows (PowerShell installer)**:

```powershell
iwr -useb <RELEASE_BASE_URL>/install.ps1 | iex
```

- **Windows (Batch wrapper, no installer EXE required)**:

```bat
install.bat
```

> The installers download a **precompiled** `chmer` for your OS/CPU. No building.
>
> Put `install.sh` and `install.ps1` in your release download directory and keep
> all precompiled archives there too.
>
> For Windows convenience, include `install.bat` and `uninstall.bat` in release assets.

## VS Code / Cursor file logo for `.ch`

To show the CHMER logo in Explorer for `.ch`/`.ctl` files:

1. Open the folder `chmer5/.vscode/chmer-icons` in VS Code/Cursor.
2. Run command: **Extensions: Install from VSIX...** (or load/install from folder depending client build).
3. Select icon theme: **CHMER Icons**.

The icon theme maps:
- `*.ch` -> CHMER logo (`../chmer.png`)
- `*.ctl` -> CHMER logo (`../chmer.png`)

## Quick start (from source)

```bash
cd chmer5
cargo build --release
./target/release/chmer
./target/release/chmer run examples/desktop_chess_app.ch
```

## CLI usage

- **Banner**:

```bash
chmer
```

- **Run a program**:

```bash
chmer run app.ch
```

- **Analyze code quality/safety**:

```bash
chmer analyze app.ch
```

## Language basics

- Statements end with `;`
- Comments start with `#`
- Imports use **exact syntax**:

```ch
(#import) chess;
(#import) gui;
(#import) inet;
```

## Desktop chess app example

See `examples/desktop_chess_app.ch`:

```ch
(#import) gui;
(#import) chess;

board = chess.board();
board.load("startpos");

func draw(ui) {
    ui.text("CHMER Desktop Chess - click piece then target");
    ui.chessboard(board);
}

gui.run("CHMER Desktop Chess", 920, 680, draw);
```

Run it:

```bash
chmer run examples/desktop_chess_app.ch
```

Installer usage examples for Linux/macOS/BSD and Windows are in:

- `examples/installers_examples.md`

## Precompiled release artifacts (what the installers expect)

Publish these files in your release (same directory as `install.sh` / `install.ps1`):

- **Linux x86_64**: `chmer-linux-x86_64.tar.gz` (contains `chmer`)
- **Linux aarch64**: `chmer-linux-aarch64.tar.gz`
- **macOS arm64**: `chmer-macos-aarch64.tar.gz`
- **macOS x86_64**: `chmer-macos-x86_64.tar.gz`
- **Windows x86_64**: `chmer-windows-x86_64.zip` (contains `chmer.exe`)
- **Windows aarch64**: `chmer-windows-aarch64.zip`
- **FreeBSD x86_64**: `chmer-freebsd-x86_64.tar.gz`
- **FreeBSD aarch64**: `chmer-freebsd-aarch64.tar.gz`
- **OpenBSD x86_64**: `chmer-openbsd-x86_64.tar.gz`
- **NetBSD x86_64**: `chmer-netbsd-x86_64.tar.gz`
- **Logo asset**: `chmer.png`
- **Unix assets pack**: `chmer-assets.tar.gz` (images/text/emoji/resources)
- **Windows assets pack**: `chmer-assets.zip`
- **Windows helpers**: `install.bat`, `uninstall.bat`

Optional convenience bundle:

- `installers.zip` (contains installer scripts + docs/assets + all platform archives currently present in `dist`)

Release packaging defaults to strict multi-platform validation. `./scripts/package_release.sh` will fail unless all platform archives above are present.

Useful overrides:

- `CHMER_REQUIRE_ALL_PLATFORMS=0 ./scripts/package_release.sh` (allow partial platform set)
- `CHMER_EXTRA_ARCHIVES_DIR=/path/to/prebuilt ./scripts/package_release.sh` (import missing `chmer-*.tar.gz` / `chmer-*.zip` before bundling)

The provided installers simply pick the right file for the platform and install into:

- **Unix**: `~/.local/bin/chmer`
- **Windows**: `$env:LOCALAPPDATA\\CHMER\\bin\\chmer.exe`

They also try to download `chmer.png` beside the binary so branding is visible in packaging/shortcuts.

## Assets support (images/text/emoji)

Installers now pull optional asset packs automatically:

- Unix/macOS/*BSD: `chmer-assets.tar.gz` -> `~/.local/share/chmer`
- Windows: `chmer-assets.zip` -> `%LOCALAPPDATA%\\CHMER\\assets`

Disable asset download:

- Unix: `CHMER_WITH_ASSETS=0`
- Windows: `$env:CHMER_WITH_ASSETS="0"`

Override asset location:

- Unix: `CHMER_ASSET_DIR=/path/to/assets`
- Windows: `$env:CHMER_ASSET_DIR="C:\\path\\to\\assets"`

## Platform strategy (Android / iOS / Web / Enterprise)

CHMER is designed to be deployed as:

- **Web backend**: `chmer run server.ch` behind Nginx/Caddy
- **Android/iOS**: embed CHMER VM in mobile shell app and load `.ch` modules
- **Desktop**: native GUI via `gui` module
- **Enterprise/corp**: package release binaries + asset packs in internal artifact repos, use `chmer analyze` in CI

Recommended enterprise setup:

1. Mirror precompiled CHMER release artifacts internally
2. Enforce `chmer analyze` in CI/CD before deployment
3. Ship approved `.ch` modules + signed asset packs
4. Run CHMER services in containers with resource limits

