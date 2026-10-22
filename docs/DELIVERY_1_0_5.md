# Local delivery 1.0.5 — 22 October 2026

Author: **LittleSunAI**. Source: the adjacent `ДЛЯ ГИТА` folder. Installer: the separate `ДЛЯ УСТАНОВКИ` folder, `SmartPhoto-Setup-1.0.5-x64.exe`. No publication or push was performed.

The update delivers the viewer, history-tree, help and final-audit fixes in [release notes](RELEASE_NOTES.md). All 609 product source files are byte-identical to the preceding audited source. Delivery changes only package versions, preparation scripts and documentation. Four README languages and the reviewed workflow illustrations are retained.

## Delivery checks

- Six version values agree on 1.0.5. Public/version/provider checks and structure checks pass in both the working project and the separate Git export. The optimized Tauri/NSIS build completed with exit code 0.
- The separate source export contains 1528 files. Its Vite/type build completed with exit code 0; all 61 frontend output files match the production frontend byte for byte. Git history is retained, no remote is configured, and build output, dependencies, user databases and credentials are excluded.
- The installer upgraded the existing 1.0.4 installation. After the final package was confirmed, a repeat 1.0.5 installation verified the exact delivered package. Both installations preserved all 849 user-profile files byte for byte before launch, along with Windows file defaults and viewer registration. Models and photographs were not removed.
- The installed EXE matches the release binary after the single documented Tauri UNK-to-NSS packaging marker. Third-party notices match the supplied source.
- The final installed binary passed a disposable-profile test: 20 actual settings states across RU/EN/DE/FR, dark surfaces, four About/version/author views, Ctrl+F after closing dialogs, real file scan and decoded viewer, 72 bidirectional cursor-anchored zoom steps starting at90/170%, menu, and normal process close/restart with preferences retained. No desktop mouse/keyboard input or paid requests were used. Release test windows were briefly created and minimized using the application's existing IPC; the debug-only hidden-window flag is not honored in production.
- Three screenshots were inspected: About after visible illustrations decoded, recognition settings, and compact viewer menu. They are local test evidence, not user photographs.

The preceding full implementation audit completed 150 isolated suites on debug candidate `a83ae5ea6da8c3bd1a3cdc5b8852312da45b0aa93095efc2a3ec4c6f0ceb0e67`, plus235 Rust tests with1 external-archive-dependent test ignored. Those results are inherited by source identity, not described as150 fresh executions on the installed binary. Paid generation, unavailable real local inference, and physical multi-monitor/DPI behavior were not repeated during delivery.

## Exact delivered files

- Installer SHA-256: `1b73c1e28d83ab230606ac8d7a9fef22f9ba6d577c3f758ee01114224feee039`
- Release EXE SHA-256: `12d6f4b1724b42dc33fb90cb8889e411d3b7c2e877826078b37f243be68a3721`
- Installed EXE SHA-256: `fdbe46c5b77521ded1da9c841fe44668d17640b932dab9ef97ec96b095247121`

`package-manifest.json` and `SHA256SUMS.txt` in the installer folder identify the local delivery. Previous installer1.0.4 was moved with hash verification to the designated intermediate-build archive onD:.

The project's own license is still pending. See the requested [MIT / Apache2.0 comparison](LICENSE_COMPARISON_RU.md). Third-party notices and full license texts remain included. The installer is unsigned; this delivery does not claim a digital publisher certificate.
