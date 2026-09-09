#!/usr/bin/env bash
set -euo pipefail

echo '--- Installing native build dependencies'
case "$(uname -s)" in
  Linux)
    node_platform=linux
    checksum_command=(sha256sum)
    privilege=()
    if ((EUID != 0)); then
      privilege=(sudo)
    fi
    "${privilege[@]}" apt-get update
    "${privilege[@]}" env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl pkg-config libssl-dev xz-utils
    ;;
  Darwin)
    node_platform=darwin
    checksum_command=(shasum -a 256)
    xcode-select -p
    brew install pkgconf openssl@3 xz
    ;;
  *) echo 'Unsupported operating system' >&2; exit 1 ;;
esac

echo '--- Installing repository development tools'
source .agents/setup

echo '--- Installing Node.js and pnpm'
case "$(uname -m)" in
  x86_64) node_architecture=x64 ;;
  aarch64|arm64) node_architecture=arm64 ;;
  *) echo 'Unsupported architecture for Node.js' >&2; exit 1 ;;
esac

tools_directory="$PWD/target/buildkite-tools"
node_archive="node-v22.23.2-${node_platform}-${node_architecture}.tar.xz"
mkdir -p "$tools_directory"
curl --proto '=https' --tlsv1.2 -fsSL "https://nodejs.org/dist/v22.23.2/$node_archive" \
  -o "$tools_directory/$node_archive"
curl --proto '=https' --tlsv1.2 -fsSL https://nodejs.org/dist/v22.23.2/SHASUMS256.txt \
  -o "$tools_directory/SHASUMS256.txt"
node_checksum=$(awk -v archive="$node_archive" '$2 == archive {print $1}' "$tools_directory/SHASUMS256.txt")
printf '%s  %s\n' "$node_checksum" "$tools_directory/$node_archive" | "${checksum_command[@]}" --check -
tar -xJf "$tools_directory/$node_archive" -C "$tools_directory" --strip-components=1
rm "$tools_directory/$node_archive" "$tools_directory/SHASUMS256.txt"

export PATH="$tools_directory/bin:$PATH"
npm install --global --prefix "$tools_directory" --no-audit --no-fund pnpm@12

cargo --version
cargo nextest --version
just --version
node --version
pnpm --version
