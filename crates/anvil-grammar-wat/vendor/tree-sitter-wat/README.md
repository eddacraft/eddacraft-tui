# Vendored: tree-sitter-wat

Generated parser for the WebAssembly **text** format (`.wat`/`.wast`), vendored
from [wasm-lsp/tree-sitter-wasm](https://github.com/wasm-lsp/tree-sitter-wasm)
(`wat/src/`), MIT licensed.

Vendored — not consumed as a crate — because the grammar is **not published to
crates.io** (LTW2-001 audit, ADR-093). `parser.c` is ABI 13 (within tree-sitter
0.26's supported 13–15 range) and has **no external scanner**, so only
`parser.c` + `tree_sitter/parser.h` are needed. Compiled by `build.rs` via `cc`.

Upstream is dormant (last commit 2022-05-17); regenerate with a current
`tree-sitter generate` if a future runtime drops ABI 13.
