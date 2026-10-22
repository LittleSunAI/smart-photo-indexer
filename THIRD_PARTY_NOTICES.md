# Third-party components

Smart Photo Indexer — prepared by LittleSunAI.

This inventory and the accompanying license texts concern third-party components. Original Smart Photo Indexer code is licensed under Apache License 2.0; see LICENSE and NOTICE. Third-party components retain the terms identified below.

Target: Windows x64. Exact versions are locked in package-lock.json and src-tauri/Cargo.lock. The inventory conservatively includes build dependencies; this does not imply every listed package is present in the installed executable. Full upstream texts are kept in third-party-licenses/.

MPL-2.0 components: the exact source releases can be obtained using the source links below. Their MPL-covered files and any modifications remain subject to MPL-2.0. Smart Photo Indexer does not replace these terms with its own author notice.

Tao 0.35.3 contains a local Windows patch, documented beside its preserved Apache license. The Microsoft WebView2 Runtime is redistributed using Microsoft’s installer and is subject to Microsoft’s terms. Ollama and model weights are downloaded separately and have their own licenses; model weights and credentials are not included in this installer.

| Component | Version | License expression | Source |
| --- | --- | --- | --- |
| Cargo: adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | [source](https://crates.io/crates/adler2/2.0.1) |
| Cargo: ahash | 0.8.12 | MIT OR Apache-2.0 | [source](https://crates.io/crates/ahash/0.8.12) |
| Cargo: aho-corasick | 1.1.4 | Unlicense OR MIT | [source](https://crates.io/crates/aho-corasick/1.1.4) |
| Cargo: alloc-no-stdlib | 2.0.4 | BSD-3-Clause | [source](https://crates.io/crates/alloc-no-stdlib/2.0.4) |
| Cargo: alloc-stdlib | 0.2.4 | BSD-3-Clause | [source](https://crates.io/crates/alloc-stdlib/0.2.4) |
| Cargo: anyhow | 1.0.103 | MIT OR Apache-2.0 | [source](https://crates.io/crates/anyhow/1.0.103) |
| Cargo: atomic-waker | 1.1.2 | Apache-2.0 OR MIT | [source](https://crates.io/crates/atomic-waker/1.1.2) |
| Cargo: autocfg | 1.5.1 | Apache-2.0 OR MIT | [source](https://crates.io/crates/autocfg/1.5.1) |
| Cargo: base64 | 0.22.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/base64/0.22.1) |
| Cargo: base64 | 0.23.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/base64/0.23.1) |
| Cargo: bit-set | 0.8.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/bit-set/0.8.0) |
| Cargo: bit-vec | 0.8.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/bit-vec/0.8.0) |
| Cargo: bitflags | 1.3.2 | MIT/Apache-2.0 | [source](https://crates.io/crates/bitflags/1.3.2) |
| Cargo: bitflags | 2.13.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/bitflags/2.13.0) |
| Cargo: block-buffer | 0.10.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/block-buffer/0.10.4) |
| Cargo: brotli | 8.0.4 | BSD-3-Clause AND MIT | [source](https://crates.io/crates/brotli/8.0.4) |
| Cargo: brotli-decompressor | 5.0.3 | BSD-3-Clause/MIT | [source](https://crates.io/crates/brotli-decompressor/5.0.3) |
| Cargo: bs58 | 0.5.1 | MIT/Apache-2.0 | [source](https://crates.io/crates/bs58/0.5.1) |
| Cargo: bumpalo | 3.20.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/bumpalo/3.20.3) |
| Cargo: bytemuck | 1.25.0 | Zlib OR Apache-2.0 OR MIT | [source](https://crates.io/crates/bytemuck/1.25.0) |
| Cargo: byteorder | 1.5.0 | Unlicense OR MIT | [source](https://crates.io/crates/byteorder/1.5.0) |
| Cargo: byteorder-lite | 0.1.0 | Unlicense OR MIT | [source](https://crates.io/crates/byteorder-lite/0.1.0) |
| Cargo: bytes | 1.12.0 | MIT | [source](https://crates.io/crates/bytes/1.12.0) |
| Cargo: camino | 1.2.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/camino/1.2.4) |
| Cargo: cargo-platform | 0.1.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cargo-platform/0.1.9) |
| Cargo: cargo_metadata | 0.19.2 | MIT | [source](https://crates.io/crates/cargo_metadata/0.19.2) |
| Cargo: cargo_toml | 0.22.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/cargo_toml/0.22.3) |
| Cargo: cc | 1.2.65 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cc/1.2.65) |
| Cargo: cfb | 0.7.3 | MIT | [source](https://crates.io/crates/cfb/0.7.3) |
| Cargo: cfg-if | 1.0.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cfg-if/1.0.4) |
| Cargo: cfg_aliases | 0.2.2 | MIT | [source](https://crates.io/crates/cfg_aliases/0.2.2) |
| Cargo: chacha20 | 0.10.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/chacha20/0.10.2) |
| Cargo: chrono | 0.4.45 | MIT OR Apache-2.0 | [source](https://crates.io/crates/chrono/0.4.45) |
| Cargo: color_quant | 1.1.0 | MIT | [source](https://crates.io/crates/color_quant/1.1.0) |
| Cargo: cookie | 0.18.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cookie/0.18.1) |
| Cargo: cpufeatures | 0.2.17 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cpufeatures/0.2.17) |
| Cargo: cpufeatures | 0.3.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/cpufeatures/0.3.1) |
| Cargo: crc32fast | 1.5.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/crc32fast/1.5.2) |
| Cargo: crossbeam-channel | 0.5.15 | MIT OR Apache-2.0 | [source](https://crates.io/crates/crossbeam-channel/0.5.15) |
| Cargo: crossbeam-utils | 0.8.21 | MIT OR Apache-2.0 | [source](https://crates.io/crates/crossbeam-utils/0.8.21) |
| Cargo: crypto-common | 0.1.7 | MIT OR Apache-2.0 | [source](https://crates.io/crates/crypto-common/0.1.7) |
| Cargo: cssparser | 0.36.0 | MPL-2.0 | [source](https://crates.io/crates/cssparser/0.36.0) |
| Cargo: cssparser-macros | 0.6.1 | MPL-2.0 | [source](https://crates.io/crates/cssparser-macros/0.6.1) |
| Cargo: ctor | 0.8.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/ctor/0.8.0) |
| Cargo: ctor-proc-macro | 0.0.7 | Apache-2.0 OR MIT | [source](https://crates.io/crates/ctor-proc-macro/0.0.7) |
| Cargo: darling | 0.23.0 | MIT | [source](https://crates.io/crates/darling/0.23.0) |
| Cargo: darling_core | 0.23.0 | MIT | [source](https://crates.io/crates/darling_core/0.23.0) |
| Cargo: darling_macro | 0.23.0 | MIT | [source](https://crates.io/crates/darling_macro/0.23.0) |
| Cargo: deranged | 0.5.8 | MIT OR Apache-2.0 | [source](https://crates.io/crates/deranged/0.5.8) |
| Cargo: derive_more | 2.1.1 | MIT | [source](https://crates.io/crates/derive_more/2.1.1) |
| Cargo: derive_more-impl | 2.1.1 | MIT | [source](https://crates.io/crates/derive_more-impl/2.1.1) |
| Cargo: digest | 0.10.7 | MIT OR Apache-2.0 | [source](https://crates.io/crates/digest/0.10.7) |
| Cargo: dirs | 6.0.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/dirs/6.0.0) |
| Cargo: dirs-sys | 0.5.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/dirs-sys/0.5.0) |
| Cargo: displaydoc | 0.2.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/displaydoc/0.2.6) |
| Cargo: dom_query | 0.27.0 | MIT | [source](https://crates.io/crates/dom_query/0.27.0) |
| Cargo: dpi | 0.1.2 | Apache-2.0 AND MIT | [source](https://crates.io/crates/dpi/0.1.2) |
| Cargo: dtoa | 1.0.11 | MIT OR Apache-2.0 | [source](https://crates.io/crates/dtoa/1.0.11) |
| Cargo: dtoa-short | 0.3.5 | MPL-2.0 | [source](https://crates.io/crates/dtoa-short/0.3.5) |
| Cargo: dtor | 0.3.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/dtor/0.3.0) |
| Cargo: dtor-proc-macro | 0.0.6 | Apache-2.0 OR MIT | [source](https://crates.io/crates/dtor-proc-macro/0.0.6) |
| Cargo: dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | [source](https://crates.io/crates/dunce/1.0.5) |
| Cargo: dyn-clone | 1.0.20 | MIT OR Apache-2.0 | [source](https://crates.io/crates/dyn-clone/1.0.20) |
| Cargo: embed-resource | 3.0.11 | MIT | [source](https://crates.io/crates/embed-resource/3.0.11) |
| Cargo: equivalent | 1.0.2 | Apache-2.0 OR MIT | [source](https://crates.io/crates/equivalent/1.0.2) |
| Cargo: erased-serde | 0.4.10 | MIT OR Apache-2.0 | [source](https://crates.io/crates/erased-serde/0.4.10) |
| Cargo: fallible-iterator | 0.3.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/fallible-iterator/0.3.0) |
| Cargo: fallible-streaming-iterator | 0.1.9 | MIT/Apache-2.0 | [source](https://crates.io/crates/fallible-streaming-iterator/0.1.9) |
| Cargo: fastrand | 2.4.1 | Apache-2.0 OR MIT | [source](https://crates.io/crates/fastrand/2.4.1) |
| Cargo: fax | 0.2.7 | MIT | [source](https://crates.io/crates/fax/0.2.7) |
| Cargo: fdeflate | 0.3.7 | MIT OR Apache-2.0 | [source](https://crates.io/crates/fdeflate/0.3.7) |
| Cargo: find-msvc-tools | 0.1.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/find-msvc-tools/0.1.9) |
| Cargo: flate2 | 1.1.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/flate2/1.1.9) |
| Cargo: fnv | 1.0.7 | Apache-2.0 / MIT | [source](https://crates.io/crates/fnv/1.0.7) |
| Cargo: foldhash | 0.2.0 | Zlib | [source](https://crates.io/crates/foldhash/0.2.0) |
| Cargo: form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/form_urlencoded/1.2.2) |
| Cargo: futures-channel | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-channel/0.3.32) |
| Cargo: futures-core | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-core/0.3.32) |
| Cargo: futures-io | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-io/0.3.32) |
| Cargo: futures-macro | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-macro/0.3.32) |
| Cargo: futures-sink | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-sink/0.3.32) |
| Cargo: futures-task | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-task/0.3.32) |
| Cargo: futures-util | 0.3.32 | MIT OR Apache-2.0 | [source](https://crates.io/crates/futures-util/0.3.32) |
| Cargo: generic-array | 0.14.7 | MIT | [source](https://crates.io/crates/generic-array/0.14.7) |
| Cargo: getrandom | 0.2.17 | MIT OR Apache-2.0 | [source](https://crates.io/crates/getrandom/0.2.17) |
| Cargo: getrandom | 0.3.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/getrandom/0.3.4) |
| Cargo: getrandom | 0.4.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/getrandom/0.4.3) |
| Cargo: gif | 0.14.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/gif/0.14.2) |
| Cargo: glob | 0.3.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/glob/0.3.3) |
| Cargo: half | 2.7.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/half/2.7.1) |
| Cargo: hashbrown | 0.12.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/hashbrown/0.12.3) |
| Cargo: hashbrown | 0.14.5 | MIT OR Apache-2.0 | [source](https://crates.io/crates/hashbrown/0.14.5) |
| Cargo: hashbrown | 0.17.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/hashbrown/0.17.1) |
| Cargo: hashlink | 0.9.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/hashlink/0.9.1) |
| Cargo: heck | 0.5.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/heck/0.5.0) |
| Cargo: hex | 0.4.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/hex/0.4.3) |
| Cargo: html5ever | 0.38.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/html5ever/0.38.0) |
| Cargo: http | 1.4.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/http/1.4.2) |
| Cargo: http-body | 1.0.1 | MIT | [source](https://crates.io/crates/http-body/1.0.1) |
| Cargo: http-body-util | 0.1.3 | MIT | [source](https://crates.io/crates/http-body-util/0.1.3) |
| Cargo: http-range | 0.1.5 | MIT | [source](https://crates.io/crates/http-range/0.1.5) |
| Cargo: httparse | 1.10.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/httparse/1.10.1) |
| Cargo: httpdate | 1.0.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/httpdate/1.0.3) |
| Cargo: hyper | 1.10.1 | MIT | [source](https://crates.io/crates/hyper/1.10.1) |
| Cargo: hyper-rustls | 0.27.10 | Apache-2.0 OR ISC OR MIT | [source](https://crates.io/crates/hyper-rustls/0.27.10) |
| Cargo: hyper-util | 0.1.20 | MIT | [source](https://crates.io/crates/hyper-util/0.1.20) |
| Cargo: ico | 0.5.0 | MIT | [source](https://crates.io/crates/ico/0.5.0) |
| Cargo: icu_collections | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_collections/2.2.0) |
| Cargo: icu_locale_core | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_locale_core/2.2.0) |
| Cargo: icu_normalizer | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_normalizer/2.2.0) |
| Cargo: icu_normalizer_data | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_normalizer_data/2.2.0) |
| Cargo: icu_properties | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_properties/2.2.0) |
| Cargo: icu_properties_data | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_properties_data/2.2.0) |
| Cargo: icu_provider | 2.2.0 | Unicode-3.0 | [source](https://crates.io/crates/icu_provider/2.2.0) |
| Cargo: ident_case | 1.0.1 | MIT/Apache-2.0 | [source](https://crates.io/crates/ident_case/1.0.1) |
| Cargo: idna | 1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/idna/1.1.0) |
| Cargo: idna_adapter | 1.2.2 | Apache-2.0 OR MIT | [source](https://crates.io/crates/idna_adapter/1.2.2) |
| Cargo: image | 0.25.10 | MIT OR Apache-2.0 | [source](https://crates.io/crates/image/0.25.10) |
| Cargo: image-webp | 0.2.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/image-webp/0.2.4) |
| Cargo: indexmap | 1.9.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/indexmap/1.9.3) |
| Cargo: indexmap | 2.14.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/indexmap/2.14.0) |
| Cargo: infer | 0.19.0 | MIT | [source](https://crates.io/crates/infer/0.19.0) |
| Cargo: ipnet | 2.12.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/ipnet/2.12.0) |
| Cargo: itoa | 1.0.18 | MIT OR Apache-2.0 | [source](https://crates.io/crates/itoa/1.0.18) |
| Cargo: json-patch | 3.0.1 | MIT/Apache-2.0 | [source](https://crates.io/crates/json-patch/3.0.1) |
| Cargo: jsonptr | 0.6.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/jsonptr/0.6.3) |
| Cargo: kamadak-exif | 0.6.1 | BSD-2-Clause | [source](https://crates.io/crates/kamadak-exif/0.6.1) |
| Cargo: keyboard-types | 0.7.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/keyboard-types/0.7.0) |
| Cargo: libc | 0.2.186 | MIT OR Apache-2.0 | [source](https://crates.io/crates/libc/0.2.186) |
| Cargo: libsqlite3-sys | 0.30.1 | MIT | [source](https://crates.io/crates/libsqlite3-sys/0.30.1) |
| Cargo: litemap | 0.8.2 | Unicode-3.0 | [source](https://crates.io/crates/litemap/0.8.2) |
| Cargo: lock_api | 0.4.14 | MIT OR Apache-2.0 | [source](https://crates.io/crates/lock_api/0.4.14) |
| Cargo: log | 0.4.34 | MIT OR Apache-2.0 | [source](https://crates.io/crates/log/0.4.34) |
| Cargo: lru-slab | 0.1.3 | MIT OR Apache-2.0 OR Zlib | [source](https://crates.io/crates/lru-slab/0.1.3) |
| Cargo: markup5ever | 0.38.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/markup5ever/0.38.0) |
| Cargo: memchr | 2.8.2 | Unlicense OR MIT | [source](https://crates.io/crates/memchr/2.8.2) |
| Cargo: mime | 0.3.17 | MIT OR Apache-2.0 | [source](https://crates.io/crates/mime/0.3.17) |
| Cargo: mime_guess | 2.0.5 | MIT | [source](https://crates.io/crates/mime_guess/2.0.5) |
| Cargo: miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | [source](https://crates.io/crates/miniz_oxide/0.8.9) |
| Cargo: mio | 1.2.1 | MIT | [source](https://crates.io/crates/mio/1.2.1) |
| Cargo: moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | [source](https://crates.io/crates/moxcms/0.8.1) |
| Cargo: muda | 0.19.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/muda/0.19.3) |
| Cargo: mutate_once | 0.1.2 | BSD-2-Clause | [source](https://crates.io/crates/mutate_once/0.1.2) |
| Cargo: new_debug_unreachable | 1.0.6 | MIT | [source](https://crates.io/crates/new_debug_unreachable/1.0.6) |
| Cargo: notify | 8.2.0 | CC0-1.0 | [source](https://crates.io/crates/notify/8.2.0) |
| Cargo: notify-types | 2.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/notify-types/2.1.0) |
| Cargo: num-conv | 0.2.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/num-conv/0.2.2) |
| Cargo: num-traits | 0.2.19 | MIT OR Apache-2.0 | [source](https://crates.io/crates/num-traits/0.2.19) |
| Cargo: once_cell | 1.21.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/once_cell/1.21.4) |
| Cargo: open | 5.3.6 | MIT | [source](https://crates.io/crates/open/5.3.6) |
| Cargo: option-ext | 0.2.0 | MPL-2.0 | [source](https://crates.io/crates/option-ext/0.2.0) |
| Cargo: parking_lot | 0.12.5 | MIT OR Apache-2.0 | [source](https://crates.io/crates/parking_lot/0.12.5) |
| Cargo: parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | [source](https://crates.io/crates/parking_lot_core/0.9.12) |
| Cargo: percent-encoding | 2.3.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/percent-encoding/2.3.2) |
| Cargo: phf | 0.13.1 | MIT | [source](https://crates.io/crates/phf/0.13.1) |
| Cargo: phf_codegen | 0.13.1 | MIT | [source](https://crates.io/crates/phf_codegen/0.13.1) |
| Cargo: phf_generator | 0.13.1 | MIT | [source](https://crates.io/crates/phf_generator/0.13.1) |
| Cargo: phf_macros | 0.13.1 | MIT | [source](https://crates.io/crates/phf_macros/0.13.1) |
| Cargo: phf_shared | 0.13.1 | MIT | [source](https://crates.io/crates/phf_shared/0.13.1) |
| Cargo: pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | [source](https://crates.io/crates/pin-project-lite/0.2.17) |
| Cargo: pkg-config | 0.3.33 | MIT OR Apache-2.0 | [source](https://crates.io/crates/pkg-config/0.3.33) |
| Cargo: plist | 1.10.1 | MIT | [source](https://crates.io/crates/plist/1.10.1) |
| Cargo: png | 0.17.16 | MIT OR Apache-2.0 | [source](https://crates.io/crates/png/0.17.16) |
| Cargo: png | 0.18.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/png/0.18.1) |
| Cargo: potential_utf | 0.1.5 | Unicode-3.0 | [source](https://crates.io/crates/potential_utf/0.1.5) |
| Cargo: powerfmt | 0.2.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/powerfmt/0.2.0) |
| Cargo: precomputed-hash | 0.1.1 | MIT | [source](https://crates.io/crates/precomputed-hash/0.1.1) |
| Cargo: proc-macro2 | 1.0.106 | MIT OR Apache-2.0 | [source](https://crates.io/crates/proc-macro2/1.0.106) |
| Cargo: pxfm | 0.1.29 | BSD-3-Clause OR Apache-2.0 | [source](https://crates.io/crates/pxfm/0.1.29) |
| Cargo: quick-error | 2.0.1 | MIT/Apache-2.0 | [source](https://crates.io/crates/quick-error/2.0.1) |
| Cargo: quick-xml | 0.42.0 | MIT | [source](https://crates.io/crates/quick-xml/0.42.0) |
| Cargo: quinn | 0.11.12 | MIT OR Apache-2.0 | [source](https://crates.io/crates/quinn/0.11.12) |
| Cargo: quinn-proto | 0.11.19 | MIT OR Apache-2.0 | [source](https://crates.io/crates/quinn-proto/0.11.19) |
| Cargo: quinn-udp | 0.5.16 | MIT OR Apache-2.0 | [source](https://crates.io/crates/quinn-udp/0.5.16) |
| Cargo: quote | 1.0.46 | MIT OR Apache-2.0 | [source](https://crates.io/crates/quote/1.0.46) |
| Cargo: rand | 0.10.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/rand/0.10.3) |
| Cargo: rand_core | 0.10.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/rand_core/0.10.1) |
| Cargo: rand_pcg | 0.10.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/rand_pcg/0.10.2) |
| Cargo: raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | [source](https://crates.io/crates/raw-window-handle/0.6.2) |
| Cargo: ref-cast | 1.0.25 | MIT OR Apache-2.0 | [source](https://crates.io/crates/ref-cast/1.0.25) |
| Cargo: ref-cast-impl | 1.0.25 | MIT OR Apache-2.0 | [source](https://crates.io/crates/ref-cast-impl/1.0.25) |
| Cargo: regex | 1.12.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/regex/1.12.4) |
| Cargo: regex-automata | 0.4.14 | MIT OR Apache-2.0 | [source](https://crates.io/crates/regex-automata/0.4.14) |
| Cargo: regex-syntax | 0.8.11 | MIT OR Apache-2.0 | [source](https://crates.io/crates/regex-syntax/0.8.11) |
| Cargo: reqwest | 0.12.28 | MIT OR Apache-2.0 | [source](https://crates.io/crates/reqwest/0.12.28) |
| Cargo: rfd | 0.15.4 | MIT | [source](https://crates.io/crates/rfd/0.15.4) |
| Cargo: ring | 0.17.14 | Apache-2.0 AND ISC | [source](https://crates.io/crates/ring/0.17.14) |
| Cargo: roxmltree | 0.21.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/roxmltree/0.21.1) |
| Cargo: rusqlite | 0.32.1 | MIT | [source](https://crates.io/crates/rusqlite/0.32.1) |
| Cargo: rust-stemmers | 1.2.0 | MIT/BSD-3-Clause | [source](https://crates.io/crates/rust-stemmers/1.2.0) |
| Cargo: rustc-hash | 2.1.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/rustc-hash/2.1.3) |
| Cargo: rustc_version | 0.4.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/rustc_version/0.4.1) |
| Cargo: rustls | 0.23.46 | Apache-2.0 OR ISC OR MIT | [source](https://crates.io/crates/rustls/0.23.46) |
| Cargo: rustls-native-certs | 0.8.4 | Apache-2.0 OR ISC OR MIT | [source](https://crates.io/crates/rustls-native-certs/0.8.4) |
| Cargo: rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/rustls-pki-types/1.15.1) |
| Cargo: rustls-webpki | 0.103.15 | ISC | [source](https://crates.io/crates/rustls-webpki/0.103.15) |
| Cargo: ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | [source](https://crates.io/crates/ryu/1.0.23) |
| Cargo: same-file | 1.0.6 | Unlicense/MIT | [source](https://crates.io/crates/same-file/1.0.6) |
| Cargo: schannel | 0.1.29 | MIT | [source](https://crates.io/crates/schannel/0.1.29) |
| Cargo: schemars | 0.8.22 | MIT | [source](https://crates.io/crates/schemars/0.8.22) |
| Cargo: schemars | 0.9.0 | MIT | [source](https://crates.io/crates/schemars/0.9.0) |
| Cargo: schemars | 1.2.1 | MIT | [source](https://crates.io/crates/schemars/1.2.1) |
| Cargo: schemars_derive | 0.8.22 | MIT | [source](https://crates.io/crates/schemars_derive/0.8.22) |
| Cargo: scopeguard | 1.2.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/scopeguard/1.2.0) |
| Cargo: selectors | 0.36.1 | MPL-2.0 | [source](https://crates.io/crates/selectors/0.36.1) |
| Cargo: semver | 1.0.28 | MIT OR Apache-2.0 | [source](https://crates.io/crates/semver/1.0.28) |
| Cargo: serde | 1.0.228 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde/1.0.228) |
| Cargo: serde-untagged | 0.1.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde-untagged/0.1.9) |
| Cargo: serde_core | 1.0.228 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_core/1.0.228) |
| Cargo: serde_derive | 1.0.228 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_derive/1.0.228) |
| Cargo: serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_derive_internals/0.29.1) |
| Cargo: serde_json | 1.0.150 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_json/1.0.150) |
| Cargo: serde_repr | 0.1.20 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_repr/0.1.20) |
| Cargo: serde_spanned | 1.1.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_spanned/1.1.1) |
| Cargo: serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | [source](https://crates.io/crates/serde_urlencoded/0.7.1) |
| Cargo: serde_with | 3.21.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_with/3.21.0) |
| Cargo: serde_with_macros | 3.21.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serde_with_macros/3.21.0) |
| Cargo: serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serialize-to-javascript/0.1.2) |
| Cargo: serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/serialize-to-javascript-impl/0.1.2) |
| Cargo: servo_arc | 0.4.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/servo_arc/0.4.3) |
| Cargo: sha2 | 0.10.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/sha2/0.10.9) |
| Cargo: shlex | 2.0.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/shlex/2.0.1) |
| Cargo: simd-adler32 | 0.3.10 | MIT | [source](https://crates.io/crates/simd-adler32/0.3.10) |
| Cargo: siphasher | 1.0.3 | MIT/Apache-2.0 | [source](https://crates.io/crates/siphasher/1.0.3) |
| Cargo: slab | 0.4.12 | MIT | [source](https://crates.io/crates/slab/0.4.12) |
| Cargo: smallvec | 1.15.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/smallvec/1.15.2) |
| Cargo: socket2 | 0.6.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/socket2/0.6.4) |
| Cargo: softbuffer | 0.4.8 | MIT OR Apache-2.0 | [source](https://crates.io/crates/softbuffer/0.4.8) |
| Cargo: stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/stable_deref_trait/1.2.1) |
| Cargo: string_cache | 0.9.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/string_cache/0.9.0) |
| Cargo: string_cache_codegen | 0.6.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/string_cache_codegen/0.6.1) |
| Cargo: strsim | 0.11.1 | MIT | [source](https://crates.io/crates/strsim/0.11.1) |
| Cargo: subtle | 2.6.1 | BSD-3-Clause | [source](https://crates.io/crates/subtle/2.6.1) |
| Cargo: syn | 2.0.118 | MIT OR Apache-2.0 | [source](https://crates.io/crates/syn/2.0.118) |
| Cargo: syn | 3.0.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/syn/3.0.6) |
| Cargo: sync_wrapper | 1.0.2 | Apache-2.0 | [source](https://crates.io/crates/sync_wrapper/1.0.2) |
| Cargo: synstructure | 0.13.2 | MIT | [source](https://crates.io/crates/synstructure/0.13.2) |
| Cargo: tao | 0.35.3 | Apache-2.0 | [source](https://crates.io/crates/tao/0.35.3) |
| Cargo: tauri | 2.11.5 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri/2.11.5) |
| Cargo: tauri-build | 2.6.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-build/2.6.3) |
| Cargo: tauri-codegen | 2.6.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-codegen/2.6.3) |
| Cargo: tauri-macros | 2.6.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-macros/2.6.3) |
| Cargo: tauri-plugin | 2.6.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-plugin/2.6.3) |
| Cargo: tauri-plugin-opener | 2.5.4 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-plugin-opener/2.5.4) |
| Cargo: tauri-plugin-single-instance | 2.4.5 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-plugin-single-instance/2.4.5) |
| Cargo: tauri-runtime | 2.11.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-runtime/2.11.3) |
| Cargo: tauri-runtime-wry | 2.11.4 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-runtime-wry/2.11.4) |
| Cargo: tauri-utils | 2.9.3 | Apache-2.0 OR MIT | [source](https://crates.io/crates/tauri-utils/2.9.3) |
| Cargo: tauri-winres | 0.3.6 | MIT | [source](https://crates.io/crates/tauri-winres/0.3.6) |
| Cargo: tendril | 0.5.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/tendril/0.5.0) |
| Cargo: thiserror | 1.0.69 | MIT OR Apache-2.0 | [source](https://crates.io/crates/thiserror/1.0.69) |
| Cargo: thiserror | 2.0.18 | MIT OR Apache-2.0 | [source](https://crates.io/crates/thiserror/2.0.18) |
| Cargo: thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | [source](https://crates.io/crates/thiserror-impl/1.0.69) |
| Cargo: thiserror-impl | 2.0.18 | MIT OR Apache-2.0 | [source](https://crates.io/crates/thiserror-impl/2.0.18) |
| Cargo: tiff | 0.11.3 | MIT | [source](https://crates.io/crates/tiff/0.11.3) |
| Cargo: time | 0.3.53 | MIT OR Apache-2.0 | [source](https://crates.io/crates/time/0.3.53) |
| Cargo: time-core | 0.1.9 | MIT OR Apache-2.0 | [source](https://crates.io/crates/time-core/0.1.9) |
| Cargo: time-macros | 0.2.31 | MIT OR Apache-2.0 | [source](https://crates.io/crates/time-macros/0.2.31) |
| Cargo: tinystr | 0.8.3 | Unicode-3.0 | [source](https://crates.io/crates/tinystr/0.8.3) |
| Cargo: tinyvec | 1.11.0 | Zlib OR Apache-2.0 OR MIT | [source](https://crates.io/crates/tinyvec/1.11.0) |
| Cargo: tinyvec_macros | 0.1.1 | MIT OR Apache-2.0 OR Zlib | [source](https://crates.io/crates/tinyvec_macros/0.1.1) |
| Cargo: tokio | 1.52.3 | MIT | [source](https://crates.io/crates/tokio/1.52.3) |
| Cargo: tokio-macros | 2.7.2 | MIT | [source](https://crates.io/crates/tokio-macros/2.7.2) |
| Cargo: tokio-rustls | 0.26.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/tokio-rustls/0.26.6) |
| Cargo: toml | 0.9.12+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml/0.9.12+spec-1.1.0) |
| Cargo: toml | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml/1.1.2+spec-1.1.0) |
| Cargo: toml_datetime | 0.7.5+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml_datetime/0.7.5+spec-1.1.0) |
| Cargo: toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml_datetime/1.1.1+spec-1.1.0) |
| Cargo: toml_parser | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml_parser/1.1.2+spec-1.1.0) |
| Cargo: toml_writer | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/toml_writer/1.1.1+spec-1.1.0) |
| Cargo: tower | 0.5.3 | MIT | [source](https://crates.io/crates/tower/0.5.3) |
| Cargo: tower-http | 0.6.11 | MIT | [source](https://crates.io/crates/tower-http/0.6.11) |
| Cargo: tower-layer | 0.3.3 | MIT | [source](https://crates.io/crates/tower-layer/0.3.3) |
| Cargo: tower-service | 0.3.3 | MIT | [source](https://crates.io/crates/tower-service/0.3.3) |
| Cargo: tracing | 0.1.44 | MIT | [source](https://crates.io/crates/tracing/0.1.44) |
| Cargo: tracing-attributes | 0.1.31 | MIT | [source](https://crates.io/crates/tracing-attributes/0.1.31) |
| Cargo: tracing-core | 0.1.36 | MIT | [source](https://crates.io/crates/tracing-core/0.1.36) |
| Cargo: trash | 5.2.6 | MIT | [source](https://crates.io/crates/trash/5.2.6) |
| Cargo: tray-icon | 0.24.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/tray-icon/0.24.1) |
| Cargo: try-lock | 0.2.5 | MIT | [source](https://crates.io/crates/try-lock/0.2.5) |
| Cargo: typed-path | 0.12.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/typed-path/0.12.3) |
| Cargo: typeid | 1.0.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/typeid/1.0.3) |
| Cargo: typenum | 1.20.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/typenum/1.20.1) |
| Cargo: unic-char-property | 0.9.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/unic-char-property/0.9.0) |
| Cargo: unic-char-range | 0.9.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/unic-char-range/0.9.0) |
| Cargo: unic-common | 0.9.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/unic-common/0.9.0) |
| Cargo: unic-ucd-ident | 0.9.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/unic-ucd-ident/0.9.0) |
| Cargo: unic-ucd-version | 0.9.0 | MIT/Apache-2.0 | [source](https://crates.io/crates/unic-ucd-version/0.9.0) |
| Cargo: unicase | 2.9.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/unicase/2.9.0) |
| Cargo: unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | [source](https://crates.io/crates/unicode-ident/1.0.24) |
| Cargo: unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/unicode-segmentation/1.13.3) |
| Cargo: untrusted | 0.9.0 | ISC | [source](https://crates.io/crates/untrusted/0.9.0) |
| Cargo: url | 2.5.8 | MIT OR Apache-2.0 | [source](https://crates.io/crates/url/2.5.8) |
| Cargo: urlpattern | 0.3.0 | MIT | [source](https://crates.io/crates/urlpattern/0.3.0) |
| Cargo: utf-8 | 0.7.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/utf-8/0.7.6) |
| Cargo: utf8_iter | 1.0.4 | Apache-2.0 OR MIT | [source](https://crates.io/crates/utf8_iter/1.0.4) |
| Cargo: uuid | 1.23.4 | Apache-2.0 OR MIT | [source](https://crates.io/crates/uuid/1.23.4) |
| Cargo: vcpkg | 0.2.15 | MIT/Apache-2.0 | [source](https://crates.io/crates/vcpkg/0.2.15) |
| Cargo: version_check | 0.9.5 | MIT/Apache-2.0 | [source](https://crates.io/crates/version_check/0.9.5) |
| Cargo: vswhom | 0.1.0 | MIT | [source](https://crates.io/crates/vswhom/0.1.0) |
| Cargo: vswhom-sys | 0.1.3 | MIT | [source](https://crates.io/crates/vswhom-sys/0.1.3) |
| Cargo: walkdir | 2.5.0 | Unlicense/MIT | [source](https://crates.io/crates/walkdir/2.5.0) |
| Cargo: want | 0.3.1 | MIT | [source](https://crates.io/crates/want/0.3.1) |
| Cargo: web_atoms | 0.2.5 | MIT OR Apache-2.0 | [source](https://crates.io/crates/web_atoms/0.2.5) |
| Cargo: webpki-roots | 1.0.9 | CDLA-Permissive-2.0 | [source](https://crates.io/crates/webpki-roots/1.0.9) |
| Cargo: webview2-com | 0.38.2 | MIT | [source](https://crates.io/crates/webview2-com/0.38.2) |
| Cargo: webview2-com-macros | 0.8.1 | MIT | [source](https://crates.io/crates/webview2-com-macros/0.8.1) |
| Cargo: webview2-com-sys | 0.38.2 | MIT | [source](https://crates.io/crates/webview2-com-sys/0.38.2) |
| Cargo: weezl | 0.1.12 | MIT OR Apache-2.0 | [source](https://crates.io/crates/weezl/0.1.12) |
| Cargo: winapi-util | 0.1.11 | Unlicense OR MIT | [source](https://crates.io/crates/winapi-util/0.1.11) |
| Cargo: window-vibrancy | 0.6.0 | Apache-2.0 OR MIT | [source](https://crates.io/crates/window-vibrancy/0.6.0) |
| Cargo: windows | 0.56.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows/0.56.0) |
| Cargo: windows | 0.61.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows/0.61.3) |
| Cargo: windows-collections | 0.2.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-collections/0.2.0) |
| Cargo: windows-core | 0.56.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-core/0.56.0) |
| Cargo: windows-core | 0.61.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-core/0.61.2) |
| Cargo: windows-future | 0.2.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-future/0.2.1) |
| Cargo: windows-implement | 0.56.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-implement/0.56.0) |
| Cargo: windows-implement | 0.60.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-implement/0.60.2) |
| Cargo: windows-interface | 0.56.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-interface/0.56.0) |
| Cargo: windows-interface | 0.59.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-interface/0.59.3) |
| Cargo: windows-link | 0.1.3 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-link/0.1.3) |
| Cargo: windows-link | 0.2.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-link/0.2.1) |
| Cargo: windows-numerics | 0.2.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-numerics/0.2.0) |
| Cargo: windows-registry | 0.6.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-registry/0.6.1) |
| Cargo: windows-result | 0.1.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-result/0.1.2) |
| Cargo: windows-result | 0.3.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-result/0.3.4) |
| Cargo: windows-result | 0.4.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-result/0.4.1) |
| Cargo: windows-strings | 0.4.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-strings/0.4.2) |
| Cargo: windows-strings | 0.5.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-strings/0.5.1) |
| Cargo: windows-sys | 0.59.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-sys/0.59.0) |
| Cargo: windows-sys | 0.60.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-sys/0.60.2) |
| Cargo: windows-sys | 0.61.2 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-sys/0.61.2) |
| Cargo: windows-targets | 0.52.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-targets/0.52.6) |
| Cargo: windows-targets | 0.53.5 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-targets/0.53.5) |
| Cargo: windows-threading | 0.1.0 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-threading/0.1.0) |
| Cargo: windows-version | 0.1.7 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows-version/0.1.7) |
| Cargo: windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows_x86_64_msvc/0.52.6) |
| Cargo: windows_x86_64_msvc | 0.53.1 | MIT OR Apache-2.0 | [source](https://crates.io/crates/windows_x86_64_msvc/0.53.1) |
| Cargo: winnow | 0.7.15 | MIT | [source](https://crates.io/crates/winnow/0.7.15) |
| Cargo: winnow | 1.0.3 | MIT | [source](https://crates.io/crates/winnow/1.0.3) |
| Cargo: winreg | 0.55.0 | MIT | [source](https://crates.io/crates/winreg/0.55.0) |
| Cargo: writeable | 0.6.3 | Unicode-3.0 | [source](https://crates.io/crates/writeable/0.6.3) |
| Cargo: wry | 0.55.1 | Apache-2.0 OR MIT | [source](https://crates.io/crates/wry/0.55.1) |
| Cargo: yoke | 0.8.3 | Unicode-3.0 | [source](https://crates.io/crates/yoke/0.8.3) |
| Cargo: yoke-derive | 0.8.2 | Unicode-3.0 | [source](https://crates.io/crates/yoke-derive/0.8.2) |
| Cargo: zerocopy | 0.8.52 | BSD-2-Clause OR Apache-2.0 OR MIT | [source](https://crates.io/crates/zerocopy/0.8.52) |
| Cargo: zerocopy-derive | 0.8.52 | BSD-2-Clause OR Apache-2.0 OR MIT | [source](https://crates.io/crates/zerocopy-derive/0.8.52) |
| Cargo: zerofrom | 0.1.8 | Unicode-3.0 | [source](https://crates.io/crates/zerofrom/0.1.8) |
| Cargo: zerofrom-derive | 0.1.7 | Unicode-3.0 | [source](https://crates.io/crates/zerofrom-derive/0.1.7) |
| Cargo: zeroize | 1.9.1 | Apache-2.0 OR MIT | [source](https://crates.io/crates/zeroize/1.9.1) |
| Cargo: zerotrie | 0.2.4 | Unicode-3.0 | [source](https://crates.io/crates/zerotrie/0.2.4) |
| Cargo: zerovec | 0.11.6 | Unicode-3.0 | [source](https://crates.io/crates/zerovec/0.11.6) |
| Cargo: zerovec-derive | 0.11.3 | Unicode-3.0 | [source](https://crates.io/crates/zerovec-derive/0.11.3) |
| Cargo: zip | 9.0.4 | MIT | [source](https://crates.io/crates/zip/9.0.4) |
| Cargo: zlib-rs | 0.6.8 | Zlib | [source](https://crates.io/crates/zlib-rs/0.6.8) |
| Cargo: zmij | 1.0.21 | MIT | [source](https://crates.io/crates/zmij/1.0.21) |
| Cargo: zopfli | 0.8.4 | Apache-2.0 | [source](https://crates.io/crates/zopfli/0.8.4) |
| Cargo: zune-core | 0.5.1 | MIT OR Apache-2.0 OR Zlib | [source](https://crates.io/crates/zune-core/0.5.1) |
| Cargo: zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | [source](https://crates.io/crates/zune-jpeg/0.5.15) |
| npm: @babel/code-frame | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/code-frame/v/7.29.7) |
| npm: @babel/compat-data | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/compat-data/v/7.29.7) |
| npm: @babel/core | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/core/v/7.29.7) |
| npm: @babel/generator | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/generator/v/7.29.7) |
| npm: @babel/helper-compilation-targets | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-compilation-targets/v/7.29.7) |
| npm: @babel/helper-globals | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-globals/v/7.29.7) |
| npm: @babel/helper-module-imports | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-module-imports/v/7.29.7) |
| npm: @babel/helper-module-transforms | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-module-transforms/v/7.29.7) |
| npm: @babel/helper-plugin-utils | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-plugin-utils/v/7.29.7) |
| npm: @babel/helper-string-parser | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-string-parser/v/7.29.7) |
| npm: @babel/helper-validator-identifier | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-validator-identifier/v/7.29.7) |
| npm: @babel/helper-validator-option | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helper-validator-option/v/7.29.7) |
| npm: @babel/helpers | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/helpers/v/7.29.7) |
| npm: @babel/parser | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/parser/v/7.29.7) |
| npm: @babel/plugin-transform-react-jsx-self | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/plugin-transform-react-jsx-self/v/7.29.7) |
| npm: @babel/plugin-transform-react-jsx-source | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/plugin-transform-react-jsx-source/v/7.29.7) |
| npm: @babel/runtime | 7.29.10 | MIT | [source](https://www.npmjs.com/package/@babel/runtime/v/7.29.10) |
| npm: @babel/template | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/template/v/7.29.7) |
| npm: @babel/traverse | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/traverse/v/7.29.7) |
| npm: @babel/types | 7.29.7 | MIT | [source](https://www.npmjs.com/package/@babel/types/v/7.29.7) |
| npm: @esbuild/win32-x64 | 0.28.1 | MIT | [source](https://www.npmjs.com/package/@esbuild/win32-x64/v/0.28.1) |
| npm: @jridgewell/gen-mapping | 0.3.13 | MIT | [source](https://www.npmjs.com/package/@jridgewell/gen-mapping/v/0.3.13) |
| npm: @jridgewell/remapping | 2.3.5 | MIT | [source](https://www.npmjs.com/package/@jridgewell/remapping/v/2.3.5) |
| npm: @jridgewell/resolve-uri | 3.1.2 | MIT | [source](https://www.npmjs.com/package/@jridgewell/resolve-uri/v/3.1.2) |
| npm: @jridgewell/sourcemap-codec | 1.5.5 | MIT | [source](https://www.npmjs.com/package/@jridgewell/sourcemap-codec/v/1.5.5) |
| npm: @jridgewell/trace-mapping | 0.3.31 | MIT | [source](https://www.npmjs.com/package/@jridgewell/trace-mapping/v/0.3.31) |
| npm: @playwright/test | 1.64.0 | Apache-2.0 | [source](https://www.npmjs.com/package/@playwright/test/v/1.64.0) |
| npm: @rolldown/pluginutils | 1.0.0-beta.27 | MIT | [source](https://www.npmjs.com/package/@rolldown/pluginutils/v/1.0.0-beta.27) |
| npm: @rollup/rollup-win32-x64-gnu | 4.62.2 | MIT | [source](https://www.npmjs.com/package/@rollup/rollup-win32-x64-gnu/v/4.62.2) |
| npm: @rollup/rollup-win32-x64-msvc | 4.62.2 | MIT | [source](https://www.npmjs.com/package/@rollup/rollup-win32-x64-msvc/v/4.62.2) |
| npm: @tauri-apps/api | 2.11.1 | Apache-2.0 OR MIT | [source](https://www.npmjs.com/package/@tauri-apps/api/v/2.11.1) |
| npm: @tauri-apps/cli | 2.11.4 | Apache-2.0 OR MIT | [source](https://www.npmjs.com/package/@tauri-apps/cli/v/2.11.4) |
| npm: @tauri-apps/cli-win32-x64-msvc | 2.11.4 | Apache-2.0 OR MIT | [source](https://www.npmjs.com/package/@tauri-apps/cli-win32-x64-msvc/v/2.11.4) |
| npm: @tauri-apps/plugin-opener | 2.5.4 | MIT OR Apache-2.0 | [source](https://www.npmjs.com/package/@tauri-apps/plugin-opener/v/2.5.4) |
| npm: @types/babel__core | 7.20.5 | MIT | [source](https://www.npmjs.com/package/@types/babel__core/v/7.20.5) |
| npm: @types/babel__generator | 7.27.0 | MIT | [source](https://www.npmjs.com/package/@types/babel__generator/v/7.27.0) |
| npm: @types/babel__template | 7.4.4 | MIT | [source](https://www.npmjs.com/package/@types/babel__template/v/7.4.4) |
| npm: @types/babel__traverse | 7.28.0 | MIT | [source](https://www.npmjs.com/package/@types/babel__traverse/v/7.28.0) |
| npm: @types/estree | 1.0.9 | MIT | [source](https://www.npmjs.com/package/@types/estree/v/1.0.9) |
| npm: @types/react | 19.2.17 | MIT | [source](https://www.npmjs.com/package/@types/react/v/19.2.17) |
| npm: @types/react-dom | 19.2.3 | MIT | [source](https://www.npmjs.com/package/@types/react-dom/v/19.2.3) |
| npm: @vitejs/plugin-react | 4.7.0 | MIT | [source](https://www.npmjs.com/package/@vitejs/plugin-react/v/4.7.0) |
| npm: baseline-browser-mapping | 2.11.29 | Apache-2.0 | [source](https://www.npmjs.com/package/baseline-browser-mapping/v/2.11.29) |
| npm: browserslist | 4.29.4 | MIT | [source](https://www.npmjs.com/package/browserslist/v/4.29.4) |
| npm: caniuse-lite | 1.0.30001816 | CC-BY-4.0 | [source](https://www.npmjs.com/package/caniuse-lite/v/1.0.30001816) |
| npm: convert-source-map | 2.0.0 | MIT | [source](https://www.npmjs.com/package/convert-source-map/v/2.0.0) |
| npm: csstype | 3.2.3 | MIT | [source](https://www.npmjs.com/package/csstype/v/3.2.3) |
| npm: debug | 4.4.3 | MIT | [source](https://www.npmjs.com/package/debug/v/4.4.3) |
| npm: electron-to-chromium | 1.5.459 | ISC | [source](https://www.npmjs.com/package/electron-to-chromium/v/1.5.459) |
| npm: esbuild | 0.28.1 | MIT | [source](https://www.npmjs.com/package/esbuild/v/0.28.1) |
| npm: escalade | 3.2.0 | MIT | [source](https://www.npmjs.com/package/escalade/v/3.2.0) |
| npm: fdir | 6.5.0 | MIT | [source](https://www.npmjs.com/package/fdir/v/6.5.0) |
| npm: gensync | 1.0.0-beta.2 | MIT | [source](https://www.npmjs.com/package/gensync/v/1.0.0-beta.2) |
| npm: html-parse-stringify | 4.0.1 | MIT | [source](https://www.npmjs.com/package/html-parse-stringify/v/4.0.1) |
| npm: i18next | 26.5.2 | MIT | [source](https://www.npmjs.com/package/i18next/v/26.5.2) |
| npm: js-tokens | 4.0.0 | MIT | [source](https://www.npmjs.com/package/js-tokens/v/4.0.0) |
| npm: jsesc | 3.1.0 | MIT | [source](https://www.npmjs.com/package/jsesc/v/3.1.0) |
| npm: json5 | 2.2.3 | MIT | [source](https://www.npmjs.com/package/json5/v/2.2.3) |
| npm: lru-cache | 5.1.1 | ISC | [source](https://www.npmjs.com/package/lru-cache/v/5.1.1) |
| npm: ms | 2.1.3 | MIT | [source](https://www.npmjs.com/package/ms/v/2.1.3) |
| npm: nanoid | 3.3.20 | MIT | [source](https://www.npmjs.com/package/nanoid/v/3.3.20) |
| npm: node-releases | 2.0.59 | MIT | [source](https://www.npmjs.com/package/node-releases/v/2.0.59) |
| npm: picocolors | 1.1.1 | ISC | [source](https://www.npmjs.com/package/picocolors/v/1.1.1) |
| npm: picomatch | 4.0.5 | MIT | [source](https://www.npmjs.com/package/picomatch/v/4.0.5) |
| npm: playwright | 1.64.0 | Apache-2.0 | [source](https://www.npmjs.com/package/playwright/v/1.64.0) |
| npm: playwright-core | 1.64.0 | Apache-2.0 | [source](https://www.npmjs.com/package/playwright-core/v/1.64.0) |
| npm: postcss | 8.5.31 | MIT | [source](https://www.npmjs.com/package/postcss/v/8.5.31) |
| npm: react | 19.2.7 | MIT | [source](https://www.npmjs.com/package/react/v/19.2.7) |
| npm: react-dom | 19.2.7 | MIT | [source](https://www.npmjs.com/package/react-dom/v/19.2.7) |
| npm: react-i18next | 17.1.0 | MIT | [source](https://www.npmjs.com/package/react-i18next/v/17.1.0) |
| npm: react-refresh | 0.17.0 | MIT | [source](https://www.npmjs.com/package/react-refresh/v/0.17.0) |
| npm: rollup | 4.62.2 | MIT | [source](https://www.npmjs.com/package/rollup/v/4.62.2) |
| npm: scheduler | 0.27.0 | MIT | [source](https://www.npmjs.com/package/scheduler/v/0.27.0) |
| npm: semver | 6.3.1 | ISC | [source](https://www.npmjs.com/package/semver/v/6.3.1) |
| npm: source-map-js | 1.2.2 | BSD-3-Clause | [source](https://www.npmjs.com/package/source-map-js/v/1.2.2) |
| npm: tinyglobby | 0.2.17 | MIT | [source](https://www.npmjs.com/package/tinyglobby/v/0.2.17) |
| npm: typescript | 5.9.3 | Apache-2.0 | [source](https://www.npmjs.com/package/typescript/v/5.9.3) |
| npm: update-browserslist-db | 1.3.4 | MIT | [source](https://www.npmjs.com/package/update-browserslist-db/v/1.3.4) |
| npm: use-sync-external-store | 1.7.0 | MIT | [source](https://www.npmjs.com/package/use-sync-external-store/v/1.7.0) |
| npm: vite | 7.3.6 | MIT | [source](https://www.npmjs.com/package/vite/v/7.3.6) |
| npm: yallist | 3.1.1 | ISC | [source](https://www.npmjs.com/package/yallist/v/3.1.1) |

A component’s original copyright, attribution, and NOTICE text is preserved in its license directory. License expressions with OR offer alternatives; expressions with AND require the applicable combined conditions. This list is generated from the actual resolved package metadata.

Microsoft WebView2 distribution: https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
Ollama source and license: https://github.com/ollama/ollama

| Cargo: chrono-tz | 0.10.4 | MIT OR Apache-2.0 | [source](https://crates.io/crates/chrono-tz/0.10.4) |

| Cargo: phf | 0.12.1 | MIT | [source](https://crates.io/crates/phf/0.12.1) |

| Cargo: phf_shared | 0.12.1 | MIT | [source](https://crates.io/crates/phf_shared/0.12.1) |
