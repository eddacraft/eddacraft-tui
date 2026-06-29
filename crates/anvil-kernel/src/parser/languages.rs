use std::path::Path;

/// Languages supported by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Rust,
    Python,
    // Tail-language wave (LANGTAIL) — T1 (Parsed): grammar wired, file
    // detected, basic symbol extraction. No per-language anti-pattern
    // catalogues, suppression syntax, or policy hooks (that is T2/T3).
    Dart,
    Go,
    Java,
    Kotlin,
    CSharp,
    /// C (`.c`/`.h`). `.h` maps to C, not C++: the choice is deterministic and
    /// the C grammar parses the overwhelming majority of header content; a C++
    /// header kept in a `.h` is a documented T1 limitation, not a parse error.
    C,
    Cpp,
    /// Zig (`.zig`/`.zon`) — tail-language wave 2 (LTW2, ADR-093), T1 (Parsed).
    Zig,
    /// WebAssembly **text** format (`.wat`/`.wast`) — tail-language wave 2
    /// (LTW2, ADR-093), T1 (Parsed). The binary `.wasm` format is deliberately
    /// excluded: it is not source and never maps here. Backed by a vendored
    /// grammar (no published crate), compiled by `build.rs`.
    Wat,
}

impl Language {
    /// Determine language from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "dart" => Some(Self::Dart),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "kt" | "kts" => Some(Self::Kotlin),
            "cs" => Some(Self::CSharp),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" => Some(Self::Cpp),
            "zig" | "zon" => Some(Self::Zig),
            // WebAssembly text format only — the binary `.wasm` is not source
            // and is intentionally absent (ADR-093 §Decision point 2).
            "wat" | "wast" => Some(Self::Wat),
            _ => None,
        }
    }

    /// Get the tree-sitter language for this language.
    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Dart => tree_sitter_dart::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            // Vendored grammar (no published crate); the unsafe FFI is isolated
            // in `anvil-grammar-wat`, which exposes this safe binding so the
            // kernel keeps `forbid(unsafe_code)`.
            Self::Wat => anvil_grammar_wat::language(),
        }
    }

    /// A grammar-version fingerprint for this language's tree-sitter grammar.
    ///
    /// Used as part of the AST cache key (LANGTS-005 K2) so that a tree-sitter
    /// grammar bump can never serve a tree parsed by an older grammar on the
    /// same content hash. The fingerprint folds the grammar's ABI version with
    /// its structural counts (`node_kind_count`, `field_count`,
    /// `parse_state_count`) — any of which shifts when the grammar's `.scm` /
    /// generated tables change across a version bump — into a single `u64`.
    ///
    /// It is derived purely from the compiled `tree_sitter::Language` and is
    /// therefore deterministic for a given grammar build (same input → same
    /// output, per the determinism principle). It is a cache *discriminator*,
    /// not a semantic version: equality means "same grammar build", and that is
    /// exactly the invariant the cache needs.
    pub fn grammar_version(&self) -> u64 {
        let lang = self.ts_language();
        // Distinct FNV-1a seed from `cache::hash_content` so the two `u64`
        // cache-key fields (content hash vs grammar version) are never built
        // from the same constants — they occupy separate slots and must not be
        // accidentally interchangeable. The `Language` discriminant is folded in
        // first so variants that share one tree-sitter grammar (JavaScript/Jsx)
        // — or any two grammars that happen to share structural counts — still
        // produce distinct fingerprints.
        let mut hash: u64 = 0x517c_c1b7_2722_0a95;
        for part in [
            *self as u64,
            lang.abi_version() as u64,
            lang.node_kind_count() as u64,
            lang.field_count() as u64,
            lang.parse_state_count() as u64,
        ] {
            hash ^= part;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_versions_are_distinct_per_language() {
        // The fingerprint is a cache *discriminator*: every supported language
        // must hash to a different value, including JavaScript vs Jsx which
        // share one tree-sitter grammar (distinguished only by the folded
        // discriminant). A collision would let a cached tree for one grammar be
        // served for another on the same content hash — the exact K2 bug.
        let versions = [
            Language::TypeScript.grammar_version(),
            Language::Tsx.grammar_version(),
            Language::JavaScript.grammar_version(),
            Language::Jsx.grammar_version(),
            Language::Rust.grammar_version(),
            Language::Python.grammar_version(),
            Language::Dart.grammar_version(),
            Language::Go.grammar_version(),
            Language::Java.grammar_version(),
            Language::Kotlin.grammar_version(),
            Language::CSharp.grammar_version(),
            Language::C.grammar_version(),
            Language::Cpp.grammar_version(),
            Language::Zig.grammar_version(),
            Language::Wat.grammar_version(),
        ];
        for (i, a) in versions.iter().enumerate() {
            for b in &versions[i + 1..] {
                assert_ne!(a, b, "grammar_version collision between languages");
            }
        }
    }

    #[test]
    fn grammar_version_is_deterministic() {
        assert_eq!(
            Language::TypeScript.grammar_version(),
            Language::TypeScript.grammar_version(),
            "same grammar build must hash identically (determinism principle)"
        );
    }

    #[test]
    fn detects_typescript() {
        assert_eq!(
            Language::from_path(Path::new("src/main.ts")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detects_tsx() {
        assert_eq!(
            Language::from_path(Path::new("App.tsx")),
            Some(Language::Tsx)
        );
    }

    #[test]
    fn detects_javascript_variants() {
        assert_eq!(
            Language::from_path(Path::new("index.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("config.mjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("require.cjs")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn detects_rust() {
        assert_eq!(
            Language::from_path(Path::new("crates/anvil-kernel/src/main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("lib.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn detects_python() {
        assert_eq!(
            Language::from_path(Path::new("pkg/mod.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("stubs/types.pyi")),
            Some(Language::Python)
        );
    }

    #[test]
    fn detects_tail_wave_languages() {
        // LANGTAIL-002..007: each tail-wave extension maps to its language.
        let cases = [
            ("lib/main.dart", Language::Dart),
            ("cmd/server/main.go", Language::Go),
            ("src/App.java", Language::Java),
            ("src/Main.kt", Language::Kotlin),
            ("build.gradle.kts", Language::Kotlin),
            ("Program.cs", Language::CSharp),
            ("src/parser.c", Language::C),
            ("include/parser.h", Language::C),
            ("src/engine.cpp", Language::Cpp),
            ("src/engine.cc", Language::Cpp),
            ("src/engine.cxx", Language::Cpp),
            ("include/engine.hpp", Language::Cpp),
            ("include/engine.hh", Language::Cpp),
        ];
        for (path, expected) in cases {
            assert_eq!(
                Language::from_path(Path::new(path)),
                Some(expected),
                "extension detection for {path}"
            );
        }
    }

    #[test]
    fn detects_zig() {
        // LTW2-003: `.zig` and `.zon` map to Zig.
        assert_eq!(
            Language::from_path(Path::new("src/main.zig")),
            Some(Language::Zig)
        );
        assert_eq!(
            Language::from_path(Path::new("build.zig.zon")),
            Some(Language::Zig)
        );
    }

    #[test]
    fn detects_wat() {
        // LTW2-002: the WebAssembly *text* format maps to Wat.
        assert_eq!(
            Language::from_path(Path::new("build/out.wat")),
            Some(Language::Wat)
        );
        assert_eq!(
            Language::from_path(Path::new("tests/spec.wast")),
            Some(Language::Wat)
        );
    }

    #[test]
    fn returns_none_for_unknown() {
        assert_eq!(Language::from_path(Path::new("README.md")), None);
        assert_eq!(Language::from_path(Path::new("Cargo.toml")), None);
        // LTW2-002 boundary: the text format `.wat`/`.wast` maps to Wat, but the
        // *binary* `.wasm` format is not source and must never be detected
        // (ADR-093 §Decision point 2).
        assert_eq!(Language::from_path(Path::new("module.wasm")), None);
    }

    #[test]
    fn ltw2_grammars_bind_and_parse() {
        // LTW2-001 acceptance, pinned as a permanent regression guard for the
        // wave-2 tail grammars (mirrors `tail_wave_grammars_bind_and_parse`):
        // each must bind the tree-sitter runtime (ABI compatible) and parse a
        // representative multi-line snippet without an error tree.
        let cases: [(Language, &str); 2] = [
            (
                Language::Zig,
                "const std = @import(\"std\");\n\npub const Point = struct {\n    x: i32,\n\n    pub fn init(x: i32) Point {\n        return Point{ .x = x };\n    }\n};\n\npub fn add(a: i32, b: i32) i32 {\n    return a + b;\n}\n",
            ),
            (
                Language::Wat,
                "(module $m\n  (func $add (param $a i32) (param $b i32) (result i32)\n    local.get $a\n    local.get $b\n    i32.add)\n  (export \"add\" (func $add)))\n",
            ),
        ];
        for (lang, source) in cases {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&lang.ts_language())
                .unwrap_or_else(|e| panic!("{lang:?} grammar must bind (ABI compatible): {e}"));
            let tree = parser
                .parse(source, None)
                .unwrap_or_else(|| panic!("{lang:?} parse must yield a tree"));
            assert!(
                !tree.root_node().has_error(),
                "{lang:?}: representative snippet must parse without errors"
            );
        }
    }

    #[test]
    fn tail_wave_grammars_bind_and_parse() {
        // LANGTAIL-001 acceptance, pinned as a permanent regression guard: every
        // included tail-wave grammar must bind to the tree-sitter runtime (ABI
        // compatible) and parse a representative multi-line snippet without an
        // error tree. A grammar/ABI regression on a version bump surfaces here
        // rather than silently dropping symbols downstream. Snippets are
        // realistic multi-line source — single-line bodies trip newline-sensitive
        // grammars (Kotlin) and are not representative of real files.
        let cases: [(Language, &str); 7] = [
            (
                Language::Dart,
                "import 'dart:io';\n\nclass Greeter {\n  String hello(String name) => name;\n}\n",
            ),
            (
                Language::Go,
                "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n",
            ),
            (
                Language::Java,
                "package app;\n\nclass Greeter {\n  String hello(String name) {\n    return name;\n  }\n}\n",
            ),
            (
                Language::Kotlin,
                "package app\n\nclass Greeter(val name: String) {\n    fun hello(): String {\n        return name\n    }\n}\n",
            ),
            (
                Language::CSharp,
                "namespace App;\n\nclass Greeter {\n    public string Hello(string name) => name;\n}\n",
            ),
            (
                Language::C,
                "#include <stdio.h>\n\nint main(void) {\n    printf(\"hi\\n\");\n    return 0;\n}\n",
            ),
            (
                Language::Cpp,
                "#include <string>\n\nclass Greeter {\npublic:\n    std::string hello() {\n        return \"hi\";\n    }\n};\n",
            ),
        ];
        for (lang, source) in cases {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&lang.ts_language())
                .unwrap_or_else(|e| panic!("{lang:?} grammar must bind (ABI compatible): {e}"));
            let tree = parser
                .parse(source, None)
                .unwrap_or_else(|| panic!("{lang:?} parse must yield a tree"));
            assert!(
                !tree.root_node().has_error(),
                "{lang:?}: representative snippet must parse without errors"
            );
        }
    }

    #[test]
    fn rust_grammar_parses_real_source() {
        // RSTLAN-001: the bound grammar must produce a non-error tree for
        // representative Rust source. This is the grammar-wiring acceptance —
        // symbol extraction is RSTLAN-002. A grammar/ABI mismatch surfaces here
        // as a `set_language` failure rather than silently downstream.
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&Language::Rust.ts_language())
            .expect("tree-sitter-rust grammar must bind (ABI compatible)");
        let source = r"
            use std::collections::HashMap;
            pub mod inner;

            pub fn main() {
                let _m: HashMap<String, u32> = HashMap::new();
            }
        ";
        let tree = parser.parse(source, None).expect("parse must yield a tree");
        assert!(
            !tree.root_node().has_error(),
            "well-formed Rust source must parse without errors"
        );
        assert_eq!(tree.root_node().kind(), "source_file");
    }
}
