#!/usr/bin/env bash
set -euo pipefail

echo '--- Installing native build dependencies'
case "$(uname -s)" in
  Linux)
    privilege=()
    if ((EUID != 0)); then
      privilege=(sudo)
    fi
    "${privilege[@]}" apt-get update
    "${privilege[@]}" env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl pkg-config libssl-dev xz-utils
    ;;
  Darwin)
    xcode-select -p
    brew install pkgconf openssl@3 xz
    ;;
  *) echo 'Unsupported operating system' >&2; exit 1 ;;
esac

echo '--- Installing repository development tools'
source .agents/setup
