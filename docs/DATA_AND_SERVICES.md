# Photos, data and external services

Browsing folders and ordinary viewing do not need an AI key or a model. The application stores its photo index, descriptions, tags, collections and history locally. Editor projects and intermediate frames use the editor's configured storage. Original photo folders remain the user's own files; explicit commands such as move, rename, delete or replace can change them. Read the confirmation and selected destination when using those commands.

Local recognition uses Ollama and a downloaded vision model. Local inference runs through the configured local service. Explicit installation can download the Ollama runtime and model weights; these downloads require a connection and disk space. Runtime and model terms are independent of this application's future license.

Cloud recognition and cloud editing use the selected service, such as OpenRouter or Fal AI. The service receives the prepared image inputs and relevant instructions when a cloud operation is authorized. Additional images and masks are also inputs when the selected operation uses them. Providers have their own terms, pricing, retention practices and regional availability. Service keys and balances are separate; a UI estimate is not a promise of the final charge.

Saved service keys use Windows DPAPI protection for the current Windows user. Do not put keys in bug reports, screenshots, repository files or shared profiles. Application data exports are not intended to transfer saved credentials or downloaded model weights. Photos themselves must be transferred separately when moving a library to another computer.

Descriptions, tags and AI outputs may contain mistakes. Review important information before using it, and keep backups of valuable originals. A successful request or a plausible description does not prove that every detail is correct.

Documentation screenshots use generated demonstration photographs and manually prepared sample descriptions and tags. They demonstrate the interface and workflow; they are not evidence of a paid model's accuracy or a guaranteed editing result. No personal photo library is supplied with the installer.

This document describes relevant data flows. It does not claim that Windows, Microsoft WebView2, Ollama, model registries or external providers never contact their own services. Third-party terms and notices are listed in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).
