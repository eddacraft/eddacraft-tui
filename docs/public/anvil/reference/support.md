---
id: support-reference
title: Supported platforms and languages
description: Check where anvil runs and what it can parse.
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# Supported platforms and languages

This page is generated from the release targets, parser mappings, and compiled
rule registry shipped with anvil 0.9.0-beta. “Parsing and structure only” means
anvil can build structural evidence for the language; it does not promise the
same specialised rule depth as a language with compiled rules.

## Configured release targets

These targets are configured for release builds. Check the assets attached to
your chosen [GitHub release](https://github.com/eddacraft/anvil/releases) before
assuming that every target is present in every beta.

| Platform              | Release target              |
| --------------------- | --------------------------- |
| macOS (Apple silicon) | `aarch64-apple-darwin`      |
| Windows (Arm64)       | `aarch64-pc-windows-msvc`   |
| Linux (Arm64)         | `aarch64-unknown-linux-gnu` |
| macOS (Intel)         | `x86_64-apple-darwin`       |
| Linux (x64)           | `x86_64-unknown-linux-gnu`  |
| Windows (x64)         | `x86_64-pc-windows-msvc`    |

## Language coverage

| Language         | File extensions                                              | Current depth               |
| ---------------- | ------------------------------------------------------------ | --------------------------- |
| TypeScript       | `.ts`                                                        | Compiled patterns available |
| TypeScript JSX   | `.tsx`                                                       | Compiled patterns available |
| JavaScript       | `.js`, `.mjs`, `.cjs`                                        | Compiled patterns available |
| JavaScript JSX   | `.jsx`                                                       | Compiled patterns available |
| Rust             | `.rs`                                                        | Compiled patterns available |
| Python           | `.py`                                                        | Compiled patterns available |
| Python           | `.pyi`                                                       | Parsing and structure only  |
| Dart             | `.dart`                                                      | Parsing and structure only  |
| Go               | `.go`                                                        | Compiled patterns available |
| Java             | `.java`                                                      | Compiled patterns available |
| Kotlin           | `.kt`, `.kts`                                                | Parsing and structure only  |
| C#               | `.cs`                                                        | Parsing and structure only  |
| C                | `.c`, `.h`                                                   | Parsing and structure only  |
| C++              | `.cpp`, `.cc`, `.cxx`, `.c++`, `.hpp`, `.hh`, `.hxx`, `.h++` | Parsing and structure only  |
| Zig              | `.zig`                                                       | Parsing and structure only  |
| WebAssembly text | `.wat`, `.wast`                                              | Parsing and structure only  |

## AI clients

`anvil start` and `anvil mcp install --client` configure supported AI clients
for pre-write validation. This public release documents **Cursor** and **Claude
Code** on the protection ladder; newer betas expand the install registry — run
`anvil mcp install --help` on your binary for the full list. Other editors can
use terminal checks and save-time watching; do not assume an editor extension is
installed.
