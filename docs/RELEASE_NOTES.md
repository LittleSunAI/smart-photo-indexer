# Release 1.0.6 — 22 October 2026

The author has selected Apache License 2.0. LICENSE and NOTICE identify LittleSunAI, package metadata uses Apache-2.0, and Help shows the license in all four interface languages with its authoritative English text. The Windows installer includes the license and notices. No photo-processing or editor request behavior changes in this release.

This release carries forward the viewer, history-tree, help and final-audit fixes from 1.0.5. Source and installer are distributed separately: source in the repository, installer/checksums/instructions in GitHub Releases. Physical multi-monitor/DPI behavior, real local inference and paid generations were not repeated for this licensing update.

# Prepared 1.0.5 — 22 October 2026

This maintenance update fixes viewer wheel zoom and reversals, makes viewer controls compact, and improves the edit-history tree: image-first cards, a movable/resizable preview, zoom/pan and camera space. The Help cover framing and reference scrolling are adjusted. Tutorial controls, themes and four languages remain available.

The final audit also corrected repeated source replacement, protected replacement assets during cleanup, mask selection after asynchronous updates, late credential replies, drag/drop listener cleanup, active-file reload in the viewer, and Ctrl+F ownership after dialogs or workspace changes.

The preceding implementation completed 150 isolated suites and 235 Rust tests (one external archive-dependent test was not run). Delivery 1.0.5 adds focused production-build, installer, profile-preservation and installed-UI checks; the earlier suite count does not imply 150 new runs on the installed EXE. Paid generation, physical multi-monitor/DPI behavior and unavailable local inference are not reverified by this update. See [delivery verification](DELIVERY_1_0_5.md).

This is a local preparation. No GitHub upload or original-code license selection is implied. OpenRouter remains disabled pending verification.

# Prepared 1.0.4 — 20 October 2026

This correction improves English, German and French interface wording: file and project actions, queue states, sorting, search, image dimensions, byte units and diagnostics. Russian remains available and unchanged. It does not change processing, project storage or AI requests.

All 201 tutorial routes were checked again in four languages at wide and narrow widths. Each outline must match the declared control or group; conditional fallbacks require an explanation. The operations panel, compact tools menu and brush-size field have additional inspected screenshots. Automated language checks and this editorial pass are not a claim of complete native-speaker proofreading of every article.

# Prepared 1.0.3 — 20 October 2026

This update completes the help, layout, editor history and project lifecycle changes. Help now has About, step-by-step learning and reference tabs. Explorer controls compact as a group; direct processing uses one list. The editor adds a branch history map, separate project checkpoints, periodic autosave, save-on-exit choices and protected project cleanup. Viewer fit transitions, cursor-anchored zoom, settings tabs and four-language coverage are improved.

Settings preserve their theme across tab and window changes. Repeated recognition explanations are removed, and localized names of local models are resolved correctly. OpenRouter remains disabled pending verification. This is a locally prepared update; no GitHub upload or license selection is implied. Spatial model precision remains model-dependent; the application explains its limits.

# Prepared 1.0.1 — 20 October 2026

A clean local installation exposed a stale Qwen3-VL packet pin and a slow detailed-mode default. The fast model pin, download size and four localized labels are updated without relaxing SHA verification. New profiles select the fast local preset; existing explicit choices are preserved. CPU/GPU policy and detailed model contracts are unchanged.

The runtime and diagnostic models were downloaded afresh. A generated landscape produced a saved description and 10 tags in 28 seconds with the fast model. Detailed Qwen3.5 exhausted its thinking/output budget after 177 seconds and was not applied. See [verification](LOCAL_INSTALL_VERIFICATION_2026_10_20.md). No cloud generation or GitHub publication was performed.

# Prepared 1.0.0 — 19 October 2026

OpenRouter is temporarily disabled through one shared build-time flag pending verification. Disabled selection and key controls are accompanied by an explanation in all four languages. Native readiness, job launch and inference reject disabled OpenRouter before requests; saved work and credentials remain intact. Fal AI and local providers remain available. Version identifiers and the prepared installer are now1.0.0. Nothing has been published; the original license remains pending.

# Prepared 0.38.7 beta

This is a local preparation, not a published GitHub Release.

- Shared application description and LittleSunAI author credit in the main, editor and viewer Help pages, localized for RU/EN/DE/FR.
- Third-party notices available from Help and included with the Windows installer.
- Version checks compare all package and application version files, with no hardcoded previous release.
- Updated affected build-tool dependencies within their allowed ranges.
- Four README languages, a generated cover, four workflow illustrations and three real application captures using generated demonstration photographs.
- A small portable offline test entry point and a source-only Git export. Historical private fixtures, credentials, development logs and binaries are excluded.

The existing photo-processing, editing request, queue and search contracts are preserved. The project's own license is intentionally pending. The installer is unsigned. Models and credentials are not bundled. AI descriptions and edits require review; service charges and availability remain provider-dependent.
