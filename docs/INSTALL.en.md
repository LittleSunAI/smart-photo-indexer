# Install Smart Photo Indexer 1.0.6

1. Run SmartPhoto-Setup-1.0.6-x64.exe on Windows 10/11 x64. The installer includes the Microsoft WebView2 runtime installer. Node.js, Rust and source files are not needed for normal use.
2. Before an update, close Smart Photo normally. The updater replaces the application files; keep a separate backup of valuable photo files and application data.
3. Open a folder. Viewing does not need an AI key. Local recognition requires an explicitly prepared model; cloud operations require your own service key and can incur charges.
4. Choose Russian, English, German or French in Settings. Use ? in each window for its own tour and Reference.

For Windows “Open with”, choose Smart Photo Indexer for an image. The app registers itself as an available viewer without replacing all file defaults automatically. Windows Settings controls your default app.

Verify the installer against SHA256SUMS.txt with Get-FileHash -Algorithm SHA256. The installer is currently unsigned; a Windows publisher/reputation warning is possible. Uninstall through Windows Installed apps. Models, keys and your original photo library are not included in this package.

Licensed under Apache License 2.0. LICENSE and NOTICE are installed with the application. Third-party notices are available in Help. Downloads: https://github.com/LittleSunAI/smart-photo-indexer/releases/latest

A new profile uses Fast · Qwen3-VL 2B when Local is selected, with a download of about 1.93 GB. Click Install and confirm setup to download Ollama and the model. An existing saved model choice is retained. Detailed Qwen3.5 modes take longer; the fast mode produces a short description and tags.

In version 1.0.2, new Qwen3.5 2B and 4B tasks generate detailed descriptions and tags without a separate reasoning stage. No manual switch is needed. Speed depends on your computer and image size; review the result for inaccuracies.
