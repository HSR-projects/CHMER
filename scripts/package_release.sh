#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
BIN="$ROOT_DIR/target/release/chmer"
EXTRA_ARCHIVES_DIR="${CHMER_EXTRA_ARCHIVES_DIR:-}"
REQUIRE_ALL_PLATFORMS="${CHMER_REQUIRE_ALL_PLATFORMS:-1}"

mkdir -p "$DIST_DIR"
rm -rf "$DIST_DIR"/*

if [ ! -x "$BIN" ]; then
  echo "release binary not found: $BIN"
  echo "run: cargo build --release"
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) os_slug="linux" ;;
  Darwin) os_slug="macos" ;;
  FreeBSD) os_slug="freebsd" ;;
  OpenBSD) os_slug="openbsd" ;;
  NetBSD) os_slug="netbsd" ;;
  *) os_slug="unknown" ;;
esac

case "$ARCH" in
  x86_64|amd64) arch_slug="x86_64" ;;
  aarch64|arm64) arch_slug="aarch64" ;;
  *) arch_slug="$ARCH" ;;
esac

PKG_DIR="$DIST_DIR/chmer-$os_slug-$arch_slug"
mkdir -p "$PKG_DIR/bin"
cp "$BIN" "$PKG_DIR/bin/chmer"
cp "$ROOT_DIR/README.md" "$PKG_DIR/README.md"
cp "$ROOT_DIR/LICENSE" "$PKG_DIR/LICENSE"
cp -R "$ROOT_DIR/examples" "$PKG_DIR/examples"
cp "$ROOT_DIR/install.sh" "$DIST_DIR/install.sh"
cp "$ROOT_DIR/install.ps1" "$DIST_DIR/install.ps1"
cp "$ROOT_DIR/install.bat" "$DIST_DIR/install.bat"
cp "$ROOT_DIR/uninstall.bat" "$DIST_DIR/uninstall.bat"
cp "$ROOT_DIR/../chmer.png" "$DIST_DIR/chmer.png"
cp "$ROOT_DIR/README.md" "$DIST_DIR/README.md"
cp "$ROOT_DIR/LICENSE" "$DIST_DIR/LICENSE"

# Optional VSIX
if [ -f "$ROOT_DIR/.vscode/chmer-icons/chmer-icons-0.0.1.vsix" ]; then
  cp "$ROOT_DIR/.vscode/chmer-icons/chmer-icons-0.0.1.vsix" "$DIST_DIR/"
fi

# Assets pack (images/text/emoji/resources starter bundle)
ASSET_STAGING="$DIST_DIR/chmer-assets"
mkdir -p "$ASSET_STAGING/images" "$ASSET_STAGING/text" "$ASSET_STAGING/emoji"
cp "$ROOT_DIR/../chmer.png" "$ASSET_STAGING/images/chmer.png"
cat > "$ASSET_STAGING/text/welcome.txt" <<'EOF'
Welcome to CHMER assets pack.
You can place custom images/text/emoji files in this bundle.
EOF
cat > "$ASSET_STAGING/emoji/default.txt" <<'EOF'
♔ ♕ ♖ ♗ ♘ ♙
♚ ♛ ♜ ♝ ♞ ♟
EOF

tar -czf "$DIST_DIR/chmer-assets.tar.gz" -C "$DIST_DIR" chmer-assets
if command -v zip >/dev/null 2>&1; then
  (cd "$DIST_DIR" && zip -rq "chmer-assets.zip" "chmer-assets")
fi
rm -rf "$ASSET_STAGING"

# Binary archive
tar -czf "$DIST_DIR/chmer-$os_slug-$arch_slug.tar.gz" -C "$DIST_DIR" "chmer-$os_slug-$arch_slug"
rm -rf "$PKG_DIR"

# Optionally import prebuilt platform archives from another location.
if [ -n "$EXTRA_ARCHIVES_DIR" ] && [ -d "$EXTRA_ARCHIVES_DIR" ]; then
  for f in "$EXTRA_ARCHIVES_DIR"/chmer-*.tar.gz "$EXTRA_ARCHIVES_DIR"/chmer-*.zip; do
    if [ -f "$f" ]; then
      cp "$f" "$DIST_DIR/"
    fi
  done
fi

# Validate platform coverage so releases are not Linux-only by accident.
EXPECTED_FILES="
chmer-linux-x86_64.tar.gz
chmer-linux-aarch64.tar.gz
chmer-macos-x86_64.tar.gz
chmer-macos-aarch64.tar.gz
chmer-windows-x86_64.zip
chmer-windows-aarch64.zip
chmer-freebsd-x86_64.tar.gz
chmer-freebsd-aarch64.tar.gz
chmer-openbsd-x86_64.tar.gz
chmer-netbsd-x86_64.tar.gz
"
MISSING_FILES=""
for f in $EXPECTED_FILES; do
  if [ ! -f "$DIST_DIR/$f" ]; then
    MISSING_FILES="$MISSING_FILES$f
"
  fi
done
if [ -n "$MISSING_FILES" ]; then
  echo "Missing platform archives:"
  printf "%s" "$MISSING_FILES"
  if [ "$REQUIRE_ALL_PLATFORMS" = "1" ]; then
    echo "error: missing required platform archives (set CHMER_REQUIRE_ALL_PLATFORMS=0 to bypass)"
    exit 1
  fi
fi

has_unix_binary() {
  archive="$1"
  tar -tzf "$archive" | rg -q '(^|/)chmer$'
}

has_windows_binary() {
  archive="$1"
  unzip -l "$archive" | awk '{print $4}' | rg -q '(^|/)chmer\.exe$'
}

# Every platform archive must include a real binary.
for unix_tgz in "$DIST_DIR"/chmer-linux-*.tar.gz \
                "$DIST_DIR"/chmer-macos-*.tar.gz \
                "$DIST_DIR"/chmer-freebsd-*.tar.gz \
                "$DIST_DIR"/chmer-openbsd-*.tar.gz \
                "$DIST_DIR"/chmer-netbsd-*.tar.gz
do
  if [ -f "$unix_tgz" ]; then
    if ! has_unix_binary "$unix_tgz"; then
      echo "error: $unix_tgz does not contain chmer binary"
      exit 1
    fi
  fi
done

# Windows archives must contain helper scripts so install.ps1 can delegate.
for win_zip in "$DIST_DIR"/chmer-windows-*.zip; do
  if [ -f "$win_zip" ] && command -v unzip >/dev/null 2>&1; then
    if ! has_windows_binary "$win_zip"; then
      echo "error: $win_zip does not contain chmer.exe"
      exit 1
    fi
    if ! unzip -l "$win_zip" | awk '{print $4}' | rg -q '^install\.bat$'; then
      echo "error: $win_zip does not contain install.bat"
      exit 1
    fi
    if ! unzip -l "$win_zip" | awk '{print $4}' | rg -q '^uninstall\.bat$'; then
      echo "error: $win_zip does not contain uninstall.bat"
      exit 1
    fi
  fi
done

# Bundle installers + all currently available platform archives into one zip.
if command -v zip >/dev/null 2>&1; then
  INSTALLERS_LIST="$DIST_DIR/installers.list"
  : > "$INSTALLERS_LIST"
  for f in \
    "install.sh" \
    "install.ps1" \
    "install.bat" \
    "uninstall.bat" \
    "README.md" \
    "LICENSE" \
    "chmer.png" \
    "chmer-assets.tar.gz" \
    "chmer-assets.zip"
  do
    if [ -f "$DIST_DIR/$f" ]; then
      printf "%s\n" "$f" >> "$INSTALLERS_LIST"
    fi
  done
  for f in "$DIST_DIR"/chmer-linux-*.tar.gz \
           "$DIST_DIR"/chmer-macos-*.tar.gz \
           "$DIST_DIR"/chmer-freebsd-*.tar.gz \
           "$DIST_DIR"/chmer-openbsd-*.tar.gz \
           "$DIST_DIR"/chmer-netbsd-*.tar.gz \
           "$DIST_DIR"/chmer-windows-*.zip
  do
    if [ -f "$f" ]; then
      basename "$f" >> "$INSTALLERS_LIST"
    fi
  done
  if [ -s "$INSTALLERS_LIST" ]; then
    (cd "$DIST_DIR" && zip -q -@ "installers.zip" < "installers.list")
  fi
  rm -f "$INSTALLERS_LIST"
fi

echo "Release bundle created in: $DIST_DIR"
echo "Generated files:"
ls -1 "$DIST_DIR"
