# Vendored: tree-sitter-wat

Generated parser for the WebAssembly **text** format (`.wat`/`.wast`), vendored
from [wasm-lsp/tree-sitter-wasm](https://github.com/wasm-lsp/tree-sitter-wasm)
(`wat/src/`).

- **Licence:** `Apache-2.0 WITH LLVM-exception` (see the `LICENSE` file here,
  copied verbatim from upstream; canonical full text:
  <https://www.apache.org/licenses/LICENSE-2.0> +
  <https://spdx.org/licenses/LLVM-exception.html>). The LLVM exception waives the
  object-code attribution requirement, so the compiled grammar may ship in the
  proprietary `anvil` binary (ADR-018).
- **Why vendored** (not a crate): the grammar is **not published to crates.io**
  (LTW2-001 audit, ADR-093).
- **ABI:** `parser.c` declares `LANGUAGE_VERSION 13` — within tree-sitter 0.26's
  supported range (13–15). No external scanner, so only `parser.c` +
  `tree_sitter/parser.h` are vendored. Compiled by the crate's `build.rs` (`cc`).
- **Provenance / integrity:** upstream is dormant (last commit `2022-05-17`).
  `parser.c` SHA-256:
  `c4fe66b9b5eeaf65f432afc031b2005a61d3b56cf4a06769fa6830d31a51e0a0`
  — recorded so a future audit can verify the blob was not modified in place.

Regenerate with a current `tree-sitter generate` against upstream `wat/grammar.js`
if a future runtime drops ABI 13; update the hash above when you do.
