#!/usr/bin/env sh
set -euo pipefail

# Cross-platform-ish installer for Linux/macOS (POSIX sh)
# - Builds with cargo if available
# - Installs binary to ${PREFIX:-/opt/beeg}/bin/beeg
# - Creates directories as needed; may require sudo for system paths

APP_NAME="beeg"
DEFAULT_PREFIX="/opt/beeg"
PREFIX="${PREFIX:-$DEFAULT_PREFIX}"
BIN_DIR="$PREFIX/bin"
TARGET_BIN="$BIN_DIR/$APP_NAME"

say() { printf "[install] %s\n" "$*"; }
err() { printf "[install][error] %s\n" "$*" 1>&2; }

usage() {
  cat <<EOF
Usage: PREFIX=/opt/beeg [FLAGS] ./install.sh

Flags:
  --uninstall           Remove installed binary from \"$DEFAULT_PREFIX/bin\" (or PREFIX)
  --no-build            Do not run cargo build (expect target/release/$APP_NAME)
  --from <path>         Install from given binary path instead of target/release
  --install-completions Install shell completions to \"$PREFIX/completions\"
  --shell <name>        Completion shell: bash|zsh|fish|powershell|elvish (repeatable)

Environment:
  PREFIX           Install prefix (default: $DEFAULT_PREFIX)

Examples:
  ./install.sh
  PREFIX=$DEFAULT_PREFIX ./install.sh
  PREFIX=$DEFAULT_PREFIX ./install.sh --uninstall
EOF
}

UNINSTALL=0
NO_BUILD=0
FROM_BIN=""
INSTALL_COMPLETIONS=0
SHELLS=""

while [ $# -gt 0 ]; do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    --uninstall) UNINSTALL=1 ; shift ;;
    --no-build) NO_BUILD=1 ; shift ;;
    --from) FROM_BIN=${2:-}; shift 2 || { err "--from requires a path"; exit 2; } ;;
    --install-completions) INSTALL_COMPLETIONS=1; shift ;;
    --shell) SHELLS="$SHELLS ${2:-}"; shift 2 || { err "--shell requires a value"; exit 2; } ;;
    *) err "Unknown option: $1"; usage; exit 2 ;;
  esac
done

need_sudo() {
  # Returns 0 if we likely need sudo to write BIN_DIR
  if [ -w "$BIN_DIR" ]; then
    return 1
  fi
  # If dir doesn't exist, check parent
  PARENT=$(dirname "$BIN_DIR")
  if [ -d "$PARENT" ] && [ -w "$PARENT" ]; then
    return 1
  fi
  return 0
}

do_uninstall() {
  if [ ! -e "$TARGET_BIN" ]; then
    say "Nothing to remove at $TARGET_BIN"
    exit 0
  fi
  if need_sudo; then
    say "Using sudo to remove $TARGET_BIN"
    sudo rm -f "$TARGET_BIN"
  else
    rm -f "$TARGET_BIN"
  fi
  say "Removed $TARGET_BIN"
}

if [ "$UNINSTALL" -eq 1 ]; then
  do_uninstall
  exit 0
fi

# Build the binary if needed
BIN_PATH=""
if [ -n "$FROM_BIN" ]; then
  BIN_PATH="$FROM_BIN"
elif [ "$NO_BUILD" -eq 1 ]; then
  BIN_PATH="target/release/$APP_NAME"
else
  if command -v cargo >/dev/null 2>&1; then
    say "Building release binary with cargo"
    cargo build --release
    BIN_PATH="target/release/$APP_NAME"
  else
    err "cargo not found and --no-build given; provide --from <binary> or install Rust."
    exit 2
  fi
fi

if [ ! -x "$BIN_PATH" ]; then
  err "Binary not found or not executable: $BIN_PATH"
  exit 2
fi

say "Installing to $TARGET_BIN"
if need_sudo; then
  say "Creating $BIN_DIR with sudo (if needed)"
  sudo mkdir -p "$BIN_DIR"
  sudo install -m 0755 "$BIN_PATH" "$TARGET_BIN"
else
  mkdir -p "$BIN_DIR"
  install -m 0755 "$BIN_PATH" "$TARGET_BIN"
fi

say "Installed $APP_NAME to $TARGET_BIN"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;; # already on PATH
  *)
    say "Note: $BIN_DIR is not on your PATH. You may add:"
    printf "    export PATH=%s:\$PATH\n" "$BIN_DIR"
    ;;
esac

COMPL_DIR="$PREFIX/completions"
if [ "$INSTALL_COMPLETIONS" -eq 1 ]; then
  say "Installing completions to $COMPL_DIR"
  if need_sudo; then
    sudo mkdir -p "$COMPL_DIR"
  else
    mkdir -p "$COMPL_DIR"
  fi
  if [ -z "$(printf %s "$SHELLS" | tr -d '[:space:]')" ]; then
    SHELLS="bash zsh fish powershell elvish"
  fi
  for shname in $SHELLS; do
    case "$shname" in
      bash) shflag="--shell bash" ;;
      zsh) shflag="--shell zsh" ;;
      fish) shflag="--shell fish" ;;
      powershell) shflag="--shell powershell" ;;
      elvish) shflag="--shell elvish" ;;
      *) err "Unknown shell for completions: $shname"; continue ;;
    esac
    say "Generating $shname completion"
    "$TARGET_BIN" completions $shflag --dir "$COMPL_DIR" || { err "Failed to generate $shname completion"; }
  done
  say "Completions installed in $COMPL_DIR"
fi

exit 0
