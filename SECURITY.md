# Security policy

## Supported versions

Only the latest release receives fixes. Upgrade with `brew upgrade --cask PingSunn/tap/pimsiang`.

## Reporting a vulnerability

Please do not open a public issue. Report it privately at <https://github.com/PingSunn/pimsiang/security/advisories/new>; only the maintainer can read it.

Include what is affected, how to reproduce it, and the PimSiang and macOS versions. You should get a first reply within a week. PimSiang is maintained by one person in their spare time, so a fix may take longer; you will be kept informed and credited in the release notes unless you prefer not to be.

## Scope

PimSiang hears the microphone, sees the Trigger key and pastes text into other apps, so issues around these are in scope, for example audio or History leaving the Mac, keys other than the Trigger key being recorded, or the Model download accepting a file that does not match its checksum.

The self-signed certificate and the one-time "Open Anyway" step are known and described in the README; they are not vulnerabilities on their own.
