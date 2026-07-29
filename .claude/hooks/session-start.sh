#!/bin/bash
# SessionStart hook — makes the regression gates runnable in a fresh remote session.
#
# Without this, every web session rediscovers the same two facts the hard way:
#   * `cargo check` / `cargo test` fail until the GTK/WebKit dev headers are present
#     (CLAUDE.md §1 documents this as a manual step — it is automated here instead),
#   * `npx tsc --noEmit` fails until node_modules exists.
#
# Idempotent and non-interactive. Local machines are skipped entirely.
set -euo pipefail

# Only run in Claude Code on the web; a developer's own box is already set up.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(dirname "$0")/../..}"

SUDO=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
  SUDO="sudo"
fi

# --- Rust: Tauri's system dependencies -------------------------------------
# `apt-get update` FIRST is required — a stale package index 404s on the .debs.
if ! pkg-config --exists gtk+-3.0 2>/dev/null; then
  echo "session-start: installing GTK/WebKit development headers…"
  $SUDO apt-get update -qq
  $SUDO apt-get install -y -qq \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev
else
  echo "session-start: GTK/WebKit headers already present."
fi

# --- Frontend --------------------------------------------------------------
if [ ! -d node_modules ]; then
  echo "session-start: installing npm dependencies…"
  npm install --no-audit --no-fund
else
  echo "session-start: node_modules already present."
fi

# --- Rust: pre-fetch the crate graph ---------------------------------------
# Cheap, and the container state is cached after the hook, so the first
# `cargo test` in the session skips the network. It will still cold-COMPILE
# (~6 min) — that is deliberate; a full build here would make session start
# unacceptably slow.
echo "session-start: fetching cargo dependencies…"
( cd src-tauri && cargo fetch --quiet ) || echo "session-start: cargo fetch failed (non-fatal)."

echo "session-start: ready. Regression gates (see docs/SCOREBOARD.md):"
echo "  cd src-tauri && cargo test --lib earth_ -- --nocapture"
echo "  cd src-tauri && cargo test --lib simulate_decades_reports_dynamics -- --nocapture"
echo "  cd src-tauri && cargo check"
echo "  npx tsc --noEmit"
