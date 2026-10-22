# Smart Photo Indexer

[English](README.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [Français](README.fr.md)
<a href="docs/media/cover.png"><img src="docs/media/cover.png" alt="Smart Photo Indexer by LittleSunAI" width="100%"></a>

**A photo library you can explore with words.** A Windows desktop app by **LittleSunAI** for browsing photos, creating descriptions and tags, searching saved metadata, and editing images.

**1.0.6 · Windows 10/11 x64 · RU / EN / DE / FR**

> Open-source software by **LittleSunAI**, licensed under [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) and [licensing details](docs/LICENSING_STATUS.md).

OpenRouter is temporarily disabled pending verification. Fal AI and local models remain available; saved OpenRouter settings and credentials are retained.

## What it helps with

- Explore folders in a grid, list or table; use favorites and collections to organize selections.
- Generate descriptions, tags or both, using a local Ollama model or a separately configured cloud service.
- Search saved descriptions, tags, filenames and paths in the current folder or the whole indexed library. This is text search, not a promise of vector similarity search.
- Prepare queues of photo-processing work and follow progress, errors and available retries.
- Open a dedicated viewer with zoom, pan and slideshow, or move into the image editor for local crop/resize and supported cloud edits.
- Work with batch tags, XMP sidecars, data exports and editor projects. Descriptions and AI results still need human review.
- Browse a tree of edit versions, compare frames and continue from a chosen frame. Save project history separately from the image, and review intermediate files before cleaning them up.

## A visual walkthrough

Four short illustrations explain the idea. They are designed workflow diagrams, not screenshots of a different interface. Their photos are generated examples; descriptions and tags shown are illustrative.

[![Start with a familiar photo folder](docs/media/01-your-library.png)](docs/media/01-your-library.png)

**1. Choose a folder.** Keep your files in familiar folders and select the photos you want to work on.

[![Create searchable descriptions and tags](docs/media/02-describe-and-tag.png)](docs/media/02-describe-and-tag.png)

**2. Describe and tag.** Select the output you need and choose local inference or a cloud service.

[![Search the saved contents with words](docs/media/03-search-by-words.png)](docs/media/03-search-by-words.png)

**3. Find the scene again.** Search the information already saved for your photos. No need to remember the filename.

[![View, compare and edit](docs/media/04-view-and-refine.png)](docs/media/04-view-and-refine.png)

**4. Take a closer look.** Inspect a photo, compare alternatives and use the editor when you need a change.

## The actual application

The following are real captures of the prepared application. Example descriptions and tags were entered manually so the documentation does not require a paid AI request. Open an image to inspect the full-resolution original.

[![Main photo library](docs/media/screenshot-library.png)](docs/media/screenshot-library.png)

[![Photo editor](docs/media/screenshot-editor.png)](docs/media/screenshot-editor.png)

[![Dedicated photo viewer](docs/media/screenshot-viewer.png)](docs/media/screenshot-viewer.png)

## Install and get started

Download the Windows installer from [Releases](https://github.com/LittleSunAI/smart-photo-indexer/releases/latest). Choose `SmartPhoto-Setup-1.0.6-x64.exe`; source code is available here or as GitHub Source code archives. Installer instructions and SHA256SUMS.txt are attached separately.

1. Run the Windows x64 installer. Ordinary use does not need Node.js or Rust.
2. Open a folder. Browsing and viewing work without an AI key.
3. For recognition, explicitly prepare a local model or configure your own cloud key. Cloud processing can cost money.
4. Select the result you want: description, tags or both. Review generated information before relying on it.
5. Choose the interface language in Settings. Use **?** for the local tour and searchable Reference in each window.

[Installation instructions](docs/INSTALL.en.md) · [Data and services](docs/DATA_AND_SERVICES.md)

## Build, checks and limitations

See [BUILD.md](docs/BUILD.md) for prerequisites and Windows builds. The portable default check does not read saved user keys, download models or call paid APIs.

```powershell
npm ci
npm run verify:structure
npm test
npm run build
```

The installer is currently unsigned. Models and credentials are not included. Local model memory and speed depend on the machine; cloud availability, billing and returned image sizes depend on the chosen service. Not every edit model accepts masks or additional images. Back up valuable originals before explicit file-changing operations.

## Author and third-party software

**LittleSunAI** — [AUTHORS.md](AUTHORS.md). Original project code: [Apache License 2.0](LICENSE), [NOTICE](NOTICE). Third-party components keep their own terms: [notices](THIRD_PARTY_NOTICES.md), full texts in `third-party-licenses/`.

[Report a bug / contribute](CONTRIBUTING.md) · [Security reports](SECURITY.md) · [Prepared release notes](docs/RELEASE_NOTES.md)

[Preparation audit](docs/PREPARATION_AUDIT.md)
