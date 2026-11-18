# Release bundle: no hardened runtime, identity-based requirement, localised usage string

`scripts/release.sh X.Y.Z [--dry-run]` builds `--release --target aarch64-apple-darwin`, assembles the bundle with the same `scripts/bundle.sh` that `bundle-dev.sh` uses (ADR 0019), signs it with "PimSiang Self-Signed", zips it with `ditto -c -k --keepParent`, renders `packaging/homebrew/pimsiang.rb.template` and then publishes the GitHub release and commits the cask to a local clone of the tap (`PIMSIANG_TAP_DIR`, default `~/project/homebrew-tap`). The tap commit is never pushed by the script. The version must be `X.Y.Z`, equal to `Cargo.toml` (the tag is `vX.Y.Z`, which the update check of ADR 0022 expects), on a clean tree. A missing identity stops the release: there is no ad-hoc fallback. `--dry-run` runs everything up to and including the zip, checksum and cask, and prints the `gh release create` and tap steps instead of running them.

**No hardened runtime.** The bundle is signed without `--options runtime` and without entitlements. Hardened runtime only pays off together with notarization, which is out of scope (ADR 0007); without it, Gatekeeper treats the app the same either way. The release binary links no third-party dylibs (`otool -L` shows only system libraries; ONNX Runtime is static), so library validation is not at stake, but turning the runtime on would also require the `com.apple.security.device.audio-input` entitlement for the microphone and could not be exercised here without launching the app. When notarization is adopted, revisit this and sign with `--options runtime` plus that entitlement.

**Designated requirement.** `codesign` gives a self-signed bundle the default requirement `identifier "com.pingsunn.pimsiang" and certificate leaf = H"<sha1 of the certificate>"`. It names the certificate, not the code hash, so it is identical for every build signed with the same certificate (checked with two different binaries signed with a throwaway certificate: different `CDHash`, same requirement). That is what lets the Accessibility, Input Monitoring and Microphone grants survive `brew upgrade`. The script fails if the requirement contains `cdhash` or lacks the leaf-certificate clause, prints it, and checks that it is unchanged after unzipping the archive. If the certificate is ever recreated, the leaf hash changes and every user has to grant the permissions again, so the keychain item must be backed up.

**Localised usage string.** Info.plist carries the English `NSMicrophoneUsageDescription` and `CFBundleLocalizations` lists `en` and `th`; `en.lproj/InfoPlist.strings` and `th.lproj/InfoPlist.strings` hold the English and Thai text, so the microphone prompt follows the system language. `CFBundleIconFile` is added when `packaging/macos/PimSiang.icns` exists; there is no icon yet.

## Considered options

- **Hardened runtime with the audio-input entitlement:** no benefit without notarization and an untested risk of breaking the app.
- **An explicit `-r` designated requirement:** the default is already certificate-based; a hand-written one adds a second place to get wrong.
- **Publishing the cask by pushing from the script:** the push is the one irreversible step, so it stays manual.
- **An ad-hoc fallback for releases:** every build would get a new requirement and reset users' grants (ADR 0007).

## Release safety and manual recovery

The script is bash 3.2 safe (macOS `/bin/bash`): the optional keychain argument is expanded as `${keychain_args[@]+"${keychain_args[@]}"}`. The identity check uses `security find-identity -v -p codesigning`, so an untrusted certificate does not count. In real mode the script also asks `origin` for the tag (`git ls-remote --tags`; never in `--dry-run`). If the GitHub release already exists, the script downloads its zip asset and compares the sha256 with the one just built: equal means the upload is skipped and the tap step continues, different stops the release. A tag on `origin` without a release also stops it. Zips are not byte-reproducible, so a rebuild normally differs; in that case recover by hand.

Manual recovery:

1. Release failed after `gh release create` but before the tap commit: do not re-run. Copy `target/release-dist/pimsiang.rb` to `$PIMSIANG_TAP_DIR/Casks/pimsiang.rb`, commit it as `pimsiang X.Y.Z`, and push the tap yourself.
2. Release exists but the asset is wrong: delete it with `gh release delete vX.Y.Z --cleanup-tag`, then re-run `scripts/release.sh X.Y.Z`. Once users installed it, bump the version instead.
3. Tag on `origin` without a release: delete the remote tag (`git push origin :refs/tags/vX.Y.Z`) and re-run, or create the release by hand.
4. Back up the signing certificate before the first release: export "PimSiang Self-Signed" with its private key as a `.p12` from Keychain Access (File, Export Items) to a safe place such as a password manager. Losing it changes the designated requirement and every user must grant the permissions again.
5. Make the repository public before telling anyone to `brew install`; the cask downloads from the GitHub release URL, which fails with 404 while the repository is private.
