#!/usr/bin/env bash
# Packages the Rust CLI backend and the Avalonia GUI frontend together into
# one dist/<rid>/ folder per requested platform, so a user gets both "run it
# as a CLI" and "launch it as a GUI" from a single unzip — no PATH setup, no
# env var, no separate install step. This works because
# frontend/DisasmViewer/Services/BackendLocator.cs checks for the CLI binary
# sitting next to the GUI executable before anything else.
#
# Usage:
#   scripts/package.sh [--self-contained] [rid ...]
#
# Examples:
#   scripts/package.sh                       # native host RID only
#   scripts/package.sh linux-x64 win-x64      # both, cross-compiling the Rust
#                                              # side where a target/linker is
#                                              # available
#   scripts/package.sh --self-contained win-x64
#
# Supported RIDs: linux-x64, win-x64, osx-x64, osx-arm64
#
# Notes:
#   - The .NET side (`dotnet publish -r <rid>`) cross-compiles for any RID
#     from any host, since it's IL — that part always works.
#   - The Rust side needs the matching Rust target + a cross-linker
#     installed to cross-compile. This machine ships with
#     x86_64-unknown-linux-gnu (native) and x86_64-pc-windows-gnu (via
#     mingw-w64, matching the mingw toolchain the corpus/ test binaries are
#     already built with — see README). macOS targets need osxcross or an
#     actual Mac; if the target/linker isn't available the script skips just
#     the Rust half for that RID and tells you so, rather than failing the
#     whole run — you can still build the backend for that RID on a machine
#     that has the toolchain and drop it into the resulting dist/<rid>/
#     folder by hand.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SELF_CONTAINED=false
RIDS=()

for arg in "$@"; do
  case "$arg" in
    --self-contained) SELF_CONTAINED=true ;;
    *) RIDS+=("$arg") ;;
  esac
done

if [ ${#RIDS[@]} -eq 0 ]; then
  case "$(uname -s)" in
    Linux) RIDS=("linux-x64") ;;
    Darwin) RIDS=("$( [ "$(uname -m)" = "arm64" ] && echo osx-arm64 || echo osx-x64 )") ;;
    MINGW*|MSYS*|CYGWIN*) RIDS=("win-x64") ;;
    *) echo "error: unrecognized host OS, pass an explicit RID" >&2; exit 1 ;;
  esac
fi

# .NET RID -> (Rust target triple, backend exe name)
rust_target_for() {
  case "$1" in
    linux-x64) echo "x86_64-unknown-linux-gnu" ;;
    win-x64) echo "x86_64-pc-windows-gnu" ;;
    osx-x64) echo "x86_64-apple-darwin" ;;
    osx-arm64) echo "aarch64-apple-darwin" ;;
    *) echo ""; ;;
  esac
}

backend_exe_name_for() {
  case "$1" in
    win-x64) echo "x64-disasm-cfg.exe" ;;
    *) echo "x64-disasm-cfg" ;;
  esac
}

DIST_DIR="$REPO_ROOT/dist"
mkdir -p "$DIST_DIR"

for rid in "${RIDS[@]}"; do
  echo "=== Packaging $rid ==="
  target="$(rust_target_for "$rid")"
  exe_name="$(backend_exe_name_for "$rid")"
  out_dir="$DIST_DIR/$rid"
  rm -rf "$out_dir"
  mkdir -p "$out_dir"

  # --- Rust backend ---
  backend_built=false
  if [ -z "$target" ]; then
    echo "warning: no known Rust target for RID '$rid' — skipping backend build" >&2
  elif rustup target list --installed 2>/dev/null | grep -qx "$target"; then
    echo "-- building backend for $target"
    if cargo build --release --target "$target"; then
      cp "target/$target/release/$exe_name" "$out_dir/$exe_name"
      backend_built=true
    else
      echo "warning: cargo build failed for target $target — skipping backend for $rid" >&2
    fi
  else
    echo "warning: Rust target '$target' not installed (rustup target add $target) — skipping backend for $rid" >&2
    echo "         (macOS targets typically also need osxcross or a real Mac to link.)" >&2
  fi

  # --- .NET frontend ---
  echo "-- publishing frontend for $rid (self-contained=$SELF_CONTAINED)"
  dotnet publish "$REPO_ROOT/frontend/DisasmViewer/DisasmViewer.csproj" \
    -c Release -r "$rid" --self-contained "$SELF_CONTAINED" \
    -p:PublishSingleFile=false \
    -o "$out_dir"

  if [ "$backend_built" = true ]; then
    echo "-- OK: $out_dir contains both the CLI ($exe_name) and the GUI, bundled together."
  else
    echo "-- NOTE: $out_dir has the GUI only. Build the backend for $rid separately and" >&2
    echo "         copy $exe_name into $out_dir/ to get the same bundled auto-discovery." >&2
  fi
  echo
done

echo "Done. See $DIST_DIR/<rid>/"
