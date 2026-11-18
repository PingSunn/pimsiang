#!/usr/bin/env bash
set -euo pipefail

NAME="PimSiang Self-Signed"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"
OPENSSL=/usr/bin/openssl

has_identity() {
  security find-identity -p codesigning "$KEYCHAIN" | grep -Fq "\"$NAME\""
}

is_trusted() {
  security find-identity -v -p codesigning "$KEYCHAIN" | grep -Fq "\"$NAME\""
}

trust_certificate() {
  local pem="$1"
  security add-trusted-cert -p codeSign -k "$KEYCHAIN" "$pem"
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if has_identity; then
  if ! is_trusted; then
    security find-certificate -c "$NAME" -p "$KEYCHAIN" > "$work/cert.pem"
    trust_certificate "$work/cert.pem"
  fi
  echo "$NAME"
  exit 0
fi

cat > "$work/openssl.cnf" <<CNF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ext
[dn]
CN = $NAME
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
CNF

password="$("$OPENSSL" rand -hex 16)"

"$OPENSSL" req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -config "$work/openssl.cnf" \
  -keyout "$work/key.pem" -out "$work/cert.pem" 2>/dev/null

PIMSIANG_P12_PASSWORD="$password" "$OPENSSL" pkcs12 -export \
  -inkey "$work/key.pem" -in "$work/cert.pem" \
  -name "$NAME" -out "$work/identity.p12" \
  -passout env:PIMSIANG_P12_PASSWORD

security import "$work/identity.p12" -k "$KEYCHAIN" -P "$password" \
  -T /usr/bin/codesign > /dev/null

trust_certificate "$work/cert.pem"

if ! is_trusted; then
  echo "The certificate was imported but macOS does not list it as valid for code signing yet." >&2
  echo "Open Keychain Access, find \"$NAME\" and set Trust > Code Signing to Always Trust." >&2
  exit 1
fi

echo "$NAME"
