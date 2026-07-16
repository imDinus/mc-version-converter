# Third-Party Licenses

This project depends on the open-source crates below. The list includes transitive
dependencies; every license is permissive and MIT-compatible. There are no
GPL/AGPL-family dependencies.

- Verified: 2026-07-10 (via `cargo metadata`)
- To regenerate: check the `license` field of each package in
  `cargo metadata --format-version 1`

## Direct dependencies

| Crate | Version | License | Purpose |
|---|---|---|---|
| fastnbt | 2.6.1 | MIT OR Apache-2.0 | NBT parsing/serialization |
| fastanvil | 0.32.0 | MIT OR Apache-2.0 | .mca region file I/O |
| flate2 | 1.1.9 | MIT OR Apache-2.0 | gzip/zlib compression |
| rayon | 1.12.0 | MIT OR Apache-2.0 | parallel chunk processing |
| thiserror | 2.0.18 | MIT OR Apache-2.0 | error type definitions |
| serde_json | 1.0.150 | MIT OR Apache-2.0 | block allowlist JSON parsing |
| clap | 4.6.1 | MIT OR Apache-2.0 | CLI argument parsing |

## Transitive dependencies

| Crate | Version | License |
|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| anstream | 1.0.0 | MIT OR Apache-2.0 |
| anstyle | 1.0.14 | MIT OR Apache-2.0 |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| bit_field | 0.10.3 | Apache-2.0/MIT |
| bytemuck | 1.25.0 | Zlib OR Apache-2.0 OR MIT |
| byteorder | 1.5.0 | Unlicense OR MIT |
| cesu8 | 1.1.0 | Apache-2.0/MIT |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 |
| clap_builder | 4.6.0 | MIT OR Apache-2.0 |
| clap_derive | 4.6.1 | MIT OR Apache-2.0 |
| clap_lex | 1.1.0 | MIT OR Apache-2.0 |
| color_quant | 1.1.0 | MIT |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 |
| crc32fast | 1.5.0 | MIT OR Apache-2.0 |
| crossbeam-deque | 0.8.7 | MIT OR Apache-2.0 |
| crossbeam-epoch | 0.9.20 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.22 | MIT OR Apache-2.0 |
| either | 1.16.0 | MIT OR Apache-2.0 |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| filetime | 0.2.29 | MIT/Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| heck | 0.5.0 | MIT OR Apache-2.0 |
| image | 0.23.14 | MIT |
| indexmap | 2.14.0 | Apache-2.0 OR MIT |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| libc | 0.2.186 | MIT OR Apache-2.0 |
| log | 0.4.33 | MIT OR Apache-2.0 |
| lz4_flex | 0.11.6 | MIT |
| lz4-java-wrc | 0.2.0 | MIT |
| memchr | 2.8.3 | Unlicense OR MIT |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| num_enum | 0.5.11 | BSD-3-Clause OR MIT OR Apache-2.0 |
| num_enum_derive | 0.5.11 | BSD-3-Clause OR MIT OR Apache-2.0 |
| num-integer | 0.1.46 | MIT OR Apache-2.0 |
| num-iter | 0.1.46 | MIT OR Apache-2.0 |
| num-rational | 0.3.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| proc-macro-crate | 1.3.1 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.106 | MIT OR Apache-2.0 |
| quote | 1.0.46 | MIT OR Apache-2.0 |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| serde_bytes | 0.11.19 | MIT OR Apache-2.0 |
| serde_core | 1.0.228 | MIT OR Apache-2.0 |
| serde_derive | 1.0.228 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.9 | MIT |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| strsim | 0.11.1 | MIT |
| syn | 1.0.109 / 2.0.118 | MIT OR Apache-2.0 |
| tar | 0.4.46 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.18 | MIT OR Apache-2.0 |
| toml_datetime | 0.6.11 | MIT OR Apache-2.0 |
| toml_edit | 0.19.15 | MIT OR Apache-2.0 |
| twox-hash | 1.6.3 | MIT |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| winnow | 0.5.40 | MIT |
| zmij | 1.0.21 | MIT |

## Notices

- This repository contains no code, game data, or assets from Mojang/Microsoft.
- The world format handling was written from scratch using the public technical
  documentation on the [Minecraft Wiki](https://minecraft.wiki) (no code copied).
- Version-number ↔ DataVersion mappings are factual data and not subject to copyright.
