#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
#
# Builds the .deb package from the release binary. Run after `npm ci && npm run build` in
# frontend/ and `cargo build --release` in backend/. The package is written to
# backend/target/debian/.
set -eu

server=$(cd "$(dirname "$0")/.." && pwd)
binary="$server/backend/target/release/entrylens"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$server/backend/Cargo.toml" | head -n 1)
arch=$(dpkg --print-architecture)
out="$server/backend/target/debian"
root="$out/entrylens_${version}_$arch"

[ -x "$binary" ] || { echo "missing $binary, run cargo build --release" >&2; exit 1; }
[ -f "$server/frontend/dist/index.html" ] || { echo "missing frontend build" >&2; exit 1; }
[ "$binary" -nt "$server/frontend/dist/index.html" ] ||
    { echo "$binary is older than the frontend build, run cargo build --release" >&2; exit 1; }

rm -rf "$root"
install -D -m 755 "$binary" "$root/usr/bin/entrylens"
install -D -m 644 "$server/deploy/entrylens.service" "$root/usr/lib/systemd/system/entrylens.service"
install -D -m 644 "$server/deploy/sysusers.conf" "$root/usr/lib/sysusers.d/entrylens.conf"
install -D -m 640 "$server/deploy/entrylens.toml" "$root/etc/entrylens.toml"
install -D -m 644 "$server/LICENSE" "$root/usr/share/doc/entrylens/copyright"
install -D -m 644 -t "$root/usr/share/entrylens/demo" "$server"/demo/*.sql

# Shared library dependencies of the binary, e.g. "libc6 (>= 2.39), libgcc-s1 (>= 4.2)".
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/debian"
printf 'Source: entrylens\n\nPackage: entrylens\nArchitecture: any\n' > "$tmp/debian/control"
depends=$(cd "$tmp" && dpkg-shlibdeps -O "$root/usr/bin/entrylens" | sed -n 's/^shlibs:Depends=//p')

install -d "$root/DEBIAN"
sed -e "s/@VERSION@/$version/" -e "s/@ARCH@/$arch/" -e "s/@DEPENDS@/$depends/" \
    "$server/deploy/debian/control" > "$root/DEBIAN/control"
install -m 644 "$server/deploy/debian/conffiles" "$root/DEBIAN/conffiles"
install -m 755 "$server/deploy/debian/postinst" "$server/deploy/debian/prerm" \
    "$server/deploy/debian/postrm" "$root/DEBIAN/"

dpkg-deb --root-owner-group --build "$root" "$out/entrylens_${version}_$arch.deb"
