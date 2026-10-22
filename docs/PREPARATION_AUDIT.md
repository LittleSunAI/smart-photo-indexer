# Release preparation 1.0.6 — 22 October 2026

The author selected Apache License 2.0 and authorized a public GitHub release. LICENSE, NOTICE, package metadata, the four Help languages and installer resources have been updated. Source and installer are distributed separately. See [release details](PUBLIC_RELEASE_1_0_6.md). The dated records below preserve their original verification scope and licensing/publication state.

# Update: preparation 1.0.5 — 22 October 2026

The final-audit fixes have been packaged and installed locally. The source-only Git folder is refreshed, and its61 frontend build files match production. All849 user-profile files and existing file associations were preserved by installation. A disposable-profile run exercised the installed EXE in four languages, settings, help, search shortcuts, viewer zoom/menu and restart. See the [current delivery record](DELIVERY_1_0_5.md) for exact hashes, evidence scope and limitations. Earlier sections below are historical, not a claim of repeated paid/physical-system checks. Original-code licensing remains pending; a [MIT/Apache comparison](LICENSE_COMPARISON_RU.md) is prepared.

# Update: preparation 1.0.3 — 20 October 2026

The complete help, layout, editor history and project lifecycle update has been built and installed locally. Twenty-seven isolated frontend suites, 232 Rust tests, eight native suites and an additional hidden compiled-WebView integration passed. The separate source folder builds the same 61 frontend files; its 606 product source files match the release source. The NSIS update preserved application data and Windows file associations. No user desktop input, second physical DPI, new paid generation, GitHub upload or license selection is claimed. Exact release/installer hashes are in the separate installer manifest. Earlier sections below are historical verification records.

# Update: preparation 1.0.1 — 20 October 2026

A user-authorized clean reinstall and real local recognition are verified. A stale fast-model pin was corrected and fresh profiles select the fast preset; saved choices remain intact. The final installer was installed, the result inspected, and user data restored. See [current verification](LOCAL_INSTALL_VERIFICATION_2026_10_20.md). Older sections retain their historical scope. No GitHub publication or own-license change was performed.

# Update: preparation1.0.0 — 19 October 2026

OpenRouter is temporarily disabled pending service verification, through a shared TS/Rust build-time flag. Disabled controls, localizable explanations, offline frontend checks and targeted native queue/availability tests cover this change. Saved preferences, credentials, pending tasks and receipts are retained. Fal AI and local providers remain available. The latest package version/source commit and executable hashes are in the separate installer manifest. The audit below records the accepted0.38.7 baseline; current capture versions are recorded per image in docs/media/captures.json. No own license, public push or user-app reinstall was performed.

# Local preparation audit — 19 October 2026

Prepared version: **0.38.7**, Windows x64. Author: **LittleSunAI**.

The local source package is prepared for review. Nothing was pushed, no GitHub
Release was created, and no CI execution on GitHub is being claimed. The separate
installer package contains the final file hashes and source commit identifier.

## What was checked

- Fresh Git-index checkout, fresh `npm ci`, structural verification, the portable
  offline test entry point, TypeScript and the production frontend build.
- A deliberately inconsistent version fixture is rejected. All five version
  declarations agree. The resource compiler accepts LF and CRLF without changing
  the meaning of the text or incorrectly reporting stale generated resources.
- The selected native `ui_error::tests` passed: two tests, 211 unrelated tests
  filtered out. The Windows application and NSIS installer were built from the
  source export. This is focused verification, not exhaustive regression testing.
- The shared About block was checked in RU/EN/DE/FR, in both Help tabs and at a
  narrow window size. Editor and viewer Help retain the editor draft and active
  photo. Actual captures were opened for visual review; the viewer footer remains
  a single row. The final executable was also checked on an isolated profile.
- Four READMEs and four installation instructions use local, checked links.
  Documentation supplies a cover, four workflow illustrations and three actual
  application screenshots. Two generated demo photos supply the examples.
  Prompts, hashes and capture provenance are in `docs/media/`.
- The selected source tree was scanned for credential patterns, private developer
  paths, executable/database/key files and oversized blobs. No matching private
  paths or credential patterns were found. The Git folder excludes user profiles,
  historical paid test scripts, model weights, installers and build caches.
  Pattern scanning is not a guarantee against every possible secret.
- Full third-party texts and a Windows dependency inventory cover 441 components,
  including build/development dependencies. Inventory paths and npm versions were
  checked. The installer bundles the notices and full texts; Help exposes the
  inventory. The project's own license is deliberately absent.

## Dependency audit and limits

`npm audit` reports **0 vulnerabilities**, including development dependencies,
after compatible lockfile updates. The Windows Cargo graph has **0 reported
vulnerabilities** in the RustSec snapshot used for this audit. TLS dependencies
were updated to rustls 0.23.46 and the Windows plist/XML path to plist 1.10.1 /
quick-xml 0.42.0. These address the applicable published advisories; see
[rustls advisory](https://rustsec.org/advisories/RUSTSEC-2026-0346.html) and
[XML advisory](https://rustsec.org/advisories/RUSTSEC-2026-0194.html).

The complete cross-platform Cargo.lock still reports two quick-xml 0.39.4
advisories through the Linux Wayland scanner, outside the resolved Windows graph.
Five Windows `unic-*` dependencies have maintenance warnings. Other-platform
warnings, including GTK/glib, remain in the full lockfile. No Linux/macOS release
is being offered or certified by this preparation. Audit database snapshot:
`999ec89d5d3f10f20be14750ec8c2d8b44e0ac9b` (17 October 2026).

The installer is unsigned. This preparation does not repeat paid image-editing or recognition requests from the app,
test every model, or promise byte-for-byte reproducible binaries. Existing
service, hardware and returned-image-size limitations remain applicable.
The user's installed application, profile, models and file associations were not
changed during this preparation.

## Before public distribution

1. Decide and apply the license for the project's own code and documentation.
   MIT is the preferred candidate, but has **not** been applied.
2. Set the intended public repository/contact information and Git author email.
   The local preparation commit uses `LittleSunAI <preparation@localhost>`;
   it does not guess a personal email address.
3. Configure the remote and publish only after explicit authorization. The
   prepared Git folder has no remote. Signing the Windows installer remains a
   separate release decision.

Some unchanged in-app reference illustrations are historical UI crops. Their
source versions remain recorded in `src/assets/help/manifest.json`. The About,
inspector and rename examples were refreshed for 0.38.7 with generated photos;
the three full-window README screenshots are current captures.
