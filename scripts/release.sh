#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

CERT_NAME="PimSiang Self-Signed"
BUNDLE_ID="com.pingsunn.pimsiang"
REPO="PingSunn/pimsiang"
TARGET="aarch64-apple-darwin"
DIST="target/release-dist"
TAP_DIR="${PIMSIANG_TAP_DIR:-$HOME/project/homebrew-tap}"
IDENTITY="${PIMSIANG_SIGN_IDENTITY:-$CERT_NAME}"

usage() {
  echo "usage: scripts/release.sh <X.Y.Z> [--dry-run]" >&2
  exit 2
}

fail() {
  echo "error: $*" >&2
  exit 1
}

version=""
dry_run=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    -*) usage ;;
    *) [ -z "$version" ] || usage; version="$arg" ;;
  esac
done
[ -n "$version" ] || usage

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "version must look like X.Y.Z (the tag becomes vX.Y.Z), got '$version'"
tag="v$version"

cargo_version="$(awk -F'"' '/^version *=/ { print $2; exit }' Cargo.toml)"
[ "$cargo_version" = "$version" ] || fail "Cargo.toml says $cargo_version, not $version; bump it and commit first"

[ -z "$(git status --porcelain)" ] || fail "working tree is not clean"
if git rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
  fail "tag $tag already exists"
fi
remote_tag=0
if [ "$dry_run" -eq 0 ] && git remote get-url origin > /dev/null 2>&1; then
  if [ -n "$(git ls-remote --tags origin "refs/tags/$tag")" ]; then
    remote_tag=1
  fi
fi

keychain_args=()
if [ -n "${PIMSIANG_SIGN_KEYCHAIN:-}" ]; then
  keychain_args=("$PIMSIANG_SIGN_KEYCHAIN")
fi
if ! security find-identity -v -p codesigning ${keychain_args[@]+"${keychain_args[@]}"} | grep -Fq "\"$IDENTITY\""; then
  fail "signing identity \"$IDENTITY\" not found; run scripts/make-signing-cert.sh once (releases are never ad hoc)"
fi

publish() {
  if [ "$dry_run" -eq 1 ]; then
    printf '[dry-run] would run:'
    printf ' %q' "$@"
    printf '\n'
  else
    "$@"
  fi
}

./scripts/check.sh

cargo build --release --locked -p app --bin pimsiang --target "$TARGET"
binary="target/$TARGET/release/pimsiang"
[ "$(lipo -archs "$binary")" = "arm64" ] || fail "$binary is not arm64 only"

rm -rf "$DIST"
mkdir -p "$DIST"
app="$DIST/PimSiang.app"
./scripts/bundle.sh "$binary" "$app" "$IDENTITY"

codesign --verify --strict --deep --verbose=2 "$app"
requirement="$(codesign -d -r- "$app" 2>&1 | sed -n 's/^designated => //p')"
case "$requirement" in
  *"identifier \"$BUNDLE_ID\""*"certificate leaf = H\""*) ;;
  *) fail "designated requirement is not identity-based: $requirement" ;;
esac
case "$requirement" in
  *cdhash*) fail "designated requirement pins a cdhash: $requirement" ;;
esac
echo "designated requirement: $requirement"

zip="$DIST/PimSiang-$version-arm64.zip"
ditto -c -k --keepParent "$app" "$zip"

check_dir="$(mktemp -d)"
trap 'rm -rf "$check_dir"' EXIT
ditto -x -k "$zip" "$check_dir"
codesign --verify --strict --deep "$check_dir/PimSiang.app"
[ "$(codesign -d -r- "$check_dir/PimSiang.app" 2>&1 | sed -n 's/^designated => //p')" = "$requirement" ] \
  || fail "designated requirement changed after unzip"

sha256="$(shasum -a 256 "$zip" | awk '{ print $1 }')"
echo "$sha256  $(basename "$zip")" > "$zip.sha256"

cask="$DIST/pimsiang.rb"
sed -e "s/@VERSION@/$version/" -e "s/@SHA256@/$sha256/" packaging/homebrew/pimsiang.rb.template > "$cask"
ruby -c "$cask" > /dev/null

echo "artifacts in $DIST: $(basename "$zip") sha256=$sha256"

if [ "$dry_run" -eq 0 ]; then
  [ -n "$(git branch -r --contains HEAD)" ] || fail "HEAD is not on any remote branch; push it so the tag lands on the released commit"
  [ -d "$TAP_DIR/.git" ] || fail "no local clone of PingSunn/homebrew-tap at $TAP_DIR (set PIMSIANG_TAP_DIR)"
  [ -z "$(git -C "$TAP_DIR" status --porcelain)" ] || fail "tap clone at $TAP_DIR is not clean"
fi

sha="$(git rev-parse HEAD)"
release_exists=0
if [ "$dry_run" -eq 0 ] && gh release view "$tag" --repo "$REPO" > /dev/null 2>&1; then
  release_exists=1
fi
if [ "$remote_tag" -eq 1 ] && [ "$release_exists" -eq 0 ]; then
  fail "tag $tag exists on origin but has no GitHub release; see docs/adr/0032 (manual recovery)"
fi
if [ "$release_exists" -eq 1 ]; then
  published="$(gh release download "$tag" --repo "$REPO" --pattern "$(basename "$zip")" --output - | shasum -a 256 | awk '{ print $1 }')"
  [ "$published" = "$sha256" ] || fail "release $tag already exists with a different asset (sha256 $published, built $sha256); see docs/adr/0032 (manual recovery)"
  echo "release $tag already has this exact asset; skipping upload"
else
  publish gh release create "$tag" "$zip" "$zip.sha256" \
    --repo "$REPO" --target "$sha" --title "PimSiang $version" --generate-notes
fi

if [ "$dry_run" -eq 1 ]; then
  echo "[dry-run] would run: copy $cask to $TAP_DIR/Casks/pimsiang.rb and commit it"
  echo "[dry-run] would then print: git -C $TAP_DIR push"
  exit 0
fi

mkdir -p "$TAP_DIR/Casks"
cp "$cask" "$TAP_DIR/Casks/pimsiang.rb"
git -C "$TAP_DIR" add Casks/pimsiang.rb
git -C "$TAP_DIR" commit -m "pimsiang $version"
echo "committed to the tap clone; review it and run: git -C $TAP_DIR push"
