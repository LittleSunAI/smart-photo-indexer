# Clean installation and local recognition — 20 October 2026

Current version: **1.0.2**, Windows x64. The 1.0.1 verification below is retained as history; the 1.0.2 results follow at the end. Local preparation only; no GitHub Release or push. Own license remains pending.

## Findings and changes

The old app was removed normally. Old profiles, runtime, model store and download cache were moved out of active discovery paths after a checksum-verified backup. Local installer 1.0.0 was used as explicitly requested; it was not downloaded from GitHub. A real default profile, official Ollama 0.40.3 download and a fresh Qwen3.5 model demonstrated the initial installation path.

The detailed Qwen3.5 2B request used reasoning and 3200 output tokens. It ended after 177 seconds with an incomplete response: its output budget was spent on thinking. Partial metadata was not applied. An automatic retry was stopped normally. This does not establish the cause of an earlier failure on another computer.

The fast Qwen3-VL download then exposed a stale packet pin. The official manifest changed; the existing strict check refused to download unmatched weights. Version 1.0.1 pins manifest `afc393e3572c07db0bc84aa460d928a16c75f3fbb9ca9e0dccc4fbd2dafb573e`, total **1930568531 bytes**, and updates four localized size labels. The source is the [official registry manifest](https://registry.ollama.ai/v2/library/qwen3-vl/manifests/2b-instruct-q4_K_M). SHA verification remains strict; an upstream change still requires review rather than silently accepting arbitrary weights.

New profiles select the fast local preset; valid saved choices are retained. Invalid selections fall back to the fast preset. CPU-only operation, other models’ reasoning policies, queues, paid providers and metadata contracts are unchanged.

## Observed verification

- Real UI confirmation downloaded the updated fast packet without using the quarantined model cache. Native readiness confirmed the expected digest, size and vision capability.
- The same generated alpine-lake source produced a description and **10 tags**: **27.814486 seconds** of Ollama time, **28 seconds** from attempt start to finish, 233 output tokens, zero cloud cost. The fast contract uses a smaller prepared frame and short response, so this is not an equal-contract quality benchmark against detailed Qwen3.5.
- Effective per-attempt configuration confirmed the fast model, local provider and one pass. Processing from the main button worked after a stopped earlier run. SQLite metadata was committed and the original image SHA stayed unchanged.
- Two targeted native tests passed: output-mode prompt/limit/reasoning contracts and catalog bounds. The frontend default/fallback/saved-choice/provider gate checks passed with zero network calls. Type checking, four-language resource compilation and the final Tauri/NSIS build passed. Unrelated full suites were not repeated.
- Final NSIS 1.0.1 installation exited successfully. Windows registration and native app version report 1.0.1; installed EXE differs from the build only by the normal three-byte NSS bundle marker. Final installed UI was inspected: result visible, labels readable, model ready, no page errors.
- User metadata, projects and UI settings were restored after a cold shutdown. Image descriptions, tag assignments and collection/job counts matched the backup after reopening; the newly downloaded runtime and models remained ready. Photos were not deleted.

This host has a Ryzen 7 7800X3D and 31.1 GiB RAM. The measured fast request used CPU, not GPU. A weaker physical computer was not tested; 28 seconds is not a promise for that computer. Model descriptions still require review. The optional detailed Qwen3.5 mode remains slower and its observed incomplete answer is retained in the development evidence.

Installer checksum and source commit are recorded in the separate installer package manifest. Personal data, raw task responses, backups and local diagnostic artifacts are excluded from this source package.


## Version 1.0.2 — direct responses for both Qwen3.5 presets

The application no longer treats support for reasoning as a request to enable it. New Qwen3.5 2B and 4B jobs explicitly use `think: false`; descriptions and tags keep the existing detailed contract and limits. Explicit reasoning in historical frozen configurations remains compatible. Fast and tags-only jobs remain direct. No new manual switch is needed. Four localized model explanations and installation guides match this behavior.

The official [Qwen3.5-2B model card](https://huggingface.co/Qwen/Qwen3.5-2B/blob/main/README.md) describes non-thinking as its default and warns that thinking can enter loops. The [4B card](https://huggingface.co/Qwen/Qwen3.5-4B/blob/main/README.md) supports direct responses too. [Ollama's API](https://docs.ollama.com/capabilities/thinking) controls this with the `think` parameter. This is different from merely hiding reasoning text.

Before the fix, 4B spent 262.63 seconds and all 3200 output tokens on thinking without a final answer. A controlled comparison changing only `think` completed the same detailed image request in 70.15 seconds. A separate three-image diagnostic series took 40.58, 34.21 and 51.81 seconds, 127.415 seconds overall. These are measurements of the diagnostic relay before the product change, not claims about the new executable.

After building 1.0.2, the application, managed Ollama runtime, active models and caches were again fully removed from active discovery. A normal NSIS installation, real default profile, fresh official runtime download and fresh 2B download were tested through the normal UI. No reasoning setting was manually changed. Three different reference images then completed directly with 2B in 28.76, 22.67 and 31.45 seconds; the whole native series took 83 seconds. All three ended with `stop`, no thinking content, full saved descriptions/tags, and unchanged source hashes. Automatic retries were disabled in the test profile, so the result is not a hidden retry series.

The new installed 1.0.2 also completed a direct 4B request in 67.13 seconds with reasoning false. Previously verified 4B/fast weights were reused after the fresh 2B test; this is not presented as another fresh 4B download. The test restoration initially missed Ollama's `manifests-v2` registry references; the harness was corrected and all pinned provenance blobs and payload hashes were verified. Normal UI installation already creates these files.

Targeted native tests cover both models in all three output modes and explicit legacy compatibility. Type/resource/version/provider checks and the Tauri/NSIS build passed. Installed native configuration checks and the necessary UI result/settings screenshots were reviewed. User descriptions, tag assignments, collections, projects and preferences were restored and checked against the cold backup; the new runtime/models remained ready. No cloud requests or GitHub publication were made.

Working output does not mean perfect visual accuracy. On the reference images, 2B still called remote controls televisions and invented a neck tag; 4B also made mistakes in fine details. No claim of universal quality or speed on a weak physical computer is made. This release fixes automatic reasoning and the observed incomplete-answer failure, not the inherent accuracy limit of the selected model weights.
