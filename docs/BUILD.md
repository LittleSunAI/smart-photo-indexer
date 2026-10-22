# Build and verify on Windows

Use Windows 10/11 x64, Node.js 22 or newer, npm, a current stable Rust toolchain with the `x86_64-pc-windows-msvc` target, and Visual Studio Build Tools with the C++ desktop workload and a Windows SDK. The desktop runtime uses Microsoft WebView2.

From the repository root:

```powershell
npm ci
npm run verify:structure
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib ui_error::
```

`npm test` is the portable, offline application check: version agreement and a deliberately invalid version fixture, localized About/help output, stable tutorial IDs, plural output, semantic error identity, and third-party notice files. It uses its own temporary directory. It does not read application credentials, process personal photos, download models, or call paid APIs. It is a focused check, not a claim of exhaustive application coverage. Installing dependencies and Rust crates can require network access.

To run the desktop development build:

```powershell
npm run tauri:dev
```

`npm run dev` is a browser preview. Native file, database, and Windows shell features require the Tauri application.

To produce the Windows installer:

```powershell
npm run tauri:build
```

The EXE is written to `src-tauri/target/release/`; the NSIS installer is written to `src-tauri/target/release/bundle/nsis/`. The configured installer includes the WebView2 offline installer, so it is larger than the application EXE. The first build may download build tools and the runtime installer. `scripts/build-windows.ps1 -Bundle nsis` is an alternative wrapper that also checks the repository structure.

Do not replace Tauri build with a plain `cargo build --release`: Tauri also builds and embeds the frontend. Close the EXE from the build directory normally before replacing it. Existing installations in other directories are separate.

The source export deliberately contains a small documented test entry point. Historical test runs, user profiles, private sample photographs, credentials, binaries and paid-service test scripts are not part of this repository. Synthetic media in `docs/media/demo` is used only for documentation examples and is never imported into a user's library automatically.

## Editing interface text

Change the author JSON in `src/shared/i18n/locales/{ru,en,de,fr}` and run:

```powershell
npm run help:compile
npm test
npm run build
```

Generated files are checked before the build. Do not edit them directly. App locale changes must not translate filenames, user metadata, prompts, AI results, IDs or request contracts.

## Reproducibility

Keep both lockfiles and the documented patch in `src-tauri/vendor/tao`. The version check compares package.json, its lockfile, Cargo.toml, Cargo.lock and tauri.conf.json. The same source snapshot must produce the supplied installer. Publishing and signing a release are separate actions; CI never uploads a release or calls a paid AI service.

## Temporary provider availability

`src/shared/config/provider-availability.json` is the single build-time flag used by both TypeScript and Rust. OpenRouter is currently disabled pending verification. To restore it after verification, change `openrouter` to `true`, run the focused checks and rebuild the installer. Do not remove saved credentials or rewrite jobs. Fal AI (including its `/openrouter/` gateway URL) is a separate provider and remains available. Existing paused OpenRouter work needs an explicit resume after restoration.
