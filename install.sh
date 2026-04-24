#!/usr/bin/env sh
set -eu

# CHMER precompiled installer (no build).
# Supports: Linux, macOS, FreeBSD, OpenBSD, NetBSD.

DEFAULT_BASE_URL="https://github.com/HSR-projects/chmer/releases/latest/download"
BASE_URL="${CHMER_RELEASE_BASE:-$DEFAULT_BASE_URL}"
INSTALL_DIR="${CHMER_INSTALL_DIR:-$HOME/.local/bin}"
ASSET_DIR="${CHMER_ASSET_DIR:-$HOME/.local/share/chmer}"
WITH_ASSETS="${CHMER_WITH_ASSETS:-1}"
TMP_DIR="${TMPDIR:-/tmp}/chmer-install.$$"

mkdir -p "$TMP_DIR"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

say() {
  printf "%s\n" "$*"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    say "error: required command '$1' not found"
    exit 1
  fi
}

detect_os() {
  os="$(uname -s 2>/dev/null || true)"
  case "$os" in
    Linux) echo "linux" ;;
    Darwin) echo "macos" ;;
    FreeBSD) echo "freebsd" ;;
    OpenBSD) echo "openbsd" ;;
    NetBSD) echo "netbsd" ;;
    *)
      say "error: unsupported OS: $os"
      exit 1
      ;;
  esac
}

detect_arch() {
  arch="$(uname -m 2>/dev/null || true)"
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *)
      say "error: unsupported CPU arch: $arch"
      exit 1
      ;;
  esac
}

archive_name() {
  os="$1"
  arch="$2"
  case "$os-$arch" in
    linux-x86_64) echo "chmer-linux-x86_64.tar.gz" ;;
    linux-aarch64) echo "chmer-linux-aarch64.tar.gz" ;;
    macos-x86_64) echo "chmer-macos-x86_64.tar.gz" ;;
    macos-aarch64) echo "chmer-macos-aarch64.tar.gz" ;;
    freebsd-x86_64) echo "chmer-freebsd-x86_64.tar.gz" ;;
    freebsd-aarch64) echo "chmer-freebsd-aarch64.tar.gz" ;;
    openbsd-x86_64) echo "chmer-openbsd-x86_64.tar.gz" ;;
    netbsd-x86_64) echo "chmer-netbsd-x86_64.tar.gz" ;;
    *)
      say "error: no precompiled binary for $os-$arch yet"
      exit 1
      ;;
  esac
}

download() {
  url="$1"
  out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --connect-timeout 15 -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$out" "$url"
  else
    say "error: need curl or wget to download files"
    exit 1
  fi
}

extract_archive() {
  archive="$1"
  out_dir="$2"
  mkdir -p "$out_dir"
  tar -xzf "$archive" -C "$out_dir"
}

find_binary() {
  dir="$1"
  if [ -f "$dir/chmer" ]; then
    echo "$dir/chmer"
    return 0
  fi
  if [ -f "$dir/bin/chmer" ]; then
    echo "$dir/bin/chmer"
    return 0
  fi
  for f in "$dir"/* "$dir"/*/*; do
    if [ -f "$f" ] && [ "$(basename "$f")" = "chmer" ]; then
      echo "$f"
      return 0
    fi
  done
  return 1
}

main() {
  os="$(detect_os)"
  arch="$(detect_arch)"
  file="$(archive_name "$os" "$arch")"
  url="$BASE_URL/$file"

  say "CHMER installer"
  say "Logo: chmer.png (included in release assets/docs)"
  say "Platform: $os-$arch"
  say "Download: $url"

  need_cmd tar
  mkdir -p "$INSTALL_DIR"

  archive="$TMP_DIR/$file"
  unpack="$TMP_DIR/unpack"

  download "$url" "$archive"
  extract_archive "$archive" "$unpack"

  bin_path="$(find_binary "$unpack" || true)"
  if [ -z "$bin_path" ]; then
    say "error: chmer binary not found in downloaded archive"
    exit 1
  fi

  cp "$bin_path" "$INSTALL_DIR/chmer"
  chmod +x "$INSTALL_DIR/chmer"

  # Optional logo sidecar for desktop packaging.
  logo_url="$BASE_URL/chmer.png"
  download "$logo_url" "$INSTALL_DIR/chmer.png" || true

  if [ "$WITH_ASSETS" = "1" ]; then
    mkdir -p "$ASSET_DIR"
    asset_file="chmer-assets.tar.gz"
    asset_url="$BASE_URL/$asset_file"
    asset_archive="$TMP_DIR/$asset_file"
    if download "$asset_url" "$asset_archive"; then
      tar -xzf "$asset_archive" -C "$ASSET_DIR" || true
      say "Assets installed: $ASSET_DIR (images/text/emoji packs)"
    else
      say "Assets pack not found in release (skipping): $asset_url"
    fi
  fi

  profile_file="$HOME/.profile"
  path_line="export PATH=\"$INSTALL_DIR:\$PATH\""
  if [ -f "$profile_file" ]; then
    if ! awk -v needle="$INSTALL_DIR" 'index($0, needle){found=1} END{exit found?0:1}' "$profile_file"; then
      printf "\n# CHMER installer\n%s\n" "$path_line" >> "$profile_file"
    fi
  else
    printf "# CHMER installer\n%s\n" "$path_line" > "$profile_file"
  fi

  say ""
  say "Installed: $INSTALL_DIR/chmer"
  say "PATH updated via: $profile_file"
  say "Asset dir: $ASSET_DIR"
  say "Open a new shell, then run: chmer"
}

main "$@"
