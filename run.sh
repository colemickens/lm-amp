#!/usr/bin/env bash
set -euo pipefail

export PATH=$PATH:$PWD
# export PLAYWRIGHT_BROWSERS_PATH="$(nix build --print-out-paths '/home/cole/code/nixcfg#pkgs.x86_64-linux.playwright-driver.browsers')"
# export PLAYWRIGHT_DRIVER="$(nix build --print-out-paths /home/cole/code/nixcfg#pkgs.x86_64-linux.playwright-driver)/bin/playwright"

export AMERIPRISE_USERNAME="colemickens"
export AMERIPRISE_PASSWORD="$(prs show ameriprise --first --quiet)"
export AMERIPRISE_TOTP="$(prs totp show ameriprise --quiet)"

export FIDELITY_USERNAME="colemickens"
export FIDELITY_PASSWORD="$(prs show fidelity --first --quiet)"
export FIDELITY_TOTP="$(prs totp show fidelity --quiet)"

export LM_TOKEN="$(prs show --quiet lunchmoney | grep apikey | cut -d ' ' -f 2)"

export CHROMIUM_BIN="$(nix build --print-out-paths 'github:nixos/nixpkgs?ref=nixos-unstable#legacyPackages.x86_64-linux.chromium')/bin/chromium"

ls -al $CHROMIUM_BIN

set -x
set -euo pipefail

rm -rf saved
mkdir saved
cargo run

