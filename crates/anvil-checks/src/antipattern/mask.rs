//! Mask comment and string spans in source so code-construct antipattern
//! rules (AP-*, GS-*) do not match `!`, `any`, etc. that appear inside
//! comments or string literals (GH #1914).
//!
//! **Byte-length-preserving.** Every masked character is replaced by ASCII
//! spaces equal to its UTF-8 byte length, and unmasked characters are copied
//! verbatim. So the byte offset of every unmasked character — which the
//! scanner records as `Warning.location.column` via `regex::Match::start()`
//! — is identical in the masked line. Match columns therefore stay accurate.
//!
//! **What is masked:** `//` line comments, `/* … */` block comments (across
//! lines), `'…'` / `"…"` string literals (with `\` escapes, and `\`-at-EOL
//! line continuation carried to the next line), and `/…/` regex literals
//! (so a `!` / `any` / `//` / quote inside a pattern is neither flagged nor
//! mistaken for a comment/string).
//!
//! **Template literals (backticks).** The literal *text* of a template is
//! masked (it is string content, so a `!` / `any` there is not a non-null
//! assertion or a type — GS-001 external-FP dogfood: `` `my-0!` ``,
//! `` `Stack overflow!` ``). The `${ … }` interpolation spans, which are real
//! code, are kept and scanned. Brace depth is tracked so a `}` inside the
//! interpolation's own code does not end it prematurely, and the in-text /
//! in-interpolation state carries across lines. Nested template literals
//! inside an interpolation are not separately re-lexed (their content is kept
//! as code, matching the prior pass-through behaviour).
//!
//! **Regex vs division.** `/` is ambiguous in JS/TS. A `/` is treated as the
//! start of a regex literal only when the preceding significant token implies
//! an expression position (an operator/opening punctuator, statement start,
//! or a regex-preceding keyword such as `return`). After a value (identifier,
//! `)`, `]`, number, string) a `/` is division. This is the same heuristic
//! syntax highlighters use; it is not a full parse, but it covers the common
//! `const re = /…/`, `x.match(/…/)`, and `return /…/` forms.

/// Lexer state that can carry across a line boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Carry {
    Code,
    /// Inside a `/* … */` block comment.
    BlockComment,
    /// Inside the *text* of a `` `…` `` template literal (masked).
    Template,
    /// Inside a `${ … }` interpolation of a template literal, at the carried
    /// brace depth (kept as code).
    TemplateInterp(u32),
    /// Inside a `'…'` string continued via a trailing `\` at end of line.
    SingleString,
    /// Inside a `"…"` string continued via a trailing `\` at end of line.
    DoubleString,
}

/// Mask the comment and string spans of every line, threading multi-line
/// state (block comments, template literals, line-continued strings) across
/// line boundaries.
pub(crate) fn mask_non_code_lines(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut carry = Carry::Code;
    for line in lines {
        let (masked, next) = mask_line(line, carry);
        out.push(masked);
        carry = next;
    }
    out
}

fn push_spaces(buf: &mut String, c: char) {
    for _ in 0..c.len_utf8() {
        buf.push(' ');
    }
}

/// Lexer state within a line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Code,
    LineComment,
    BlockComment,
    /// Template literal text (masked).
    Template,
    /// Inside a `${ … }` interpolation (kept as code).
    TemplateInterp,
    Single,
    Double,
    Regex,
}

/// Keywords after which a `/` begins a regex literal rather than division.
const REGEX_PRECEDING_KEYWORDS: &[&str] = &[
    "return",
    "typeof",
    "instanceof",
    "in",
    "of",
    "case",
    "delete",
    "void",
    "yield",
    "do",
    "else",
    "new",
    "throw",
    "await",
];

/// Decide whether a `/` in code position starts a regex literal (`true`) or
/// is a division operator (`false`), from the previous significant character
/// and the code emitted so far on the line.
fn regex_allowed(prev_sig: Option<char>, out_so_far: &str) -> bool {
    match prev_sig {
        // Statement / line start: a `/` here can only be a regex.
        None => true,
        Some(c) => {
            if matches!(
                c,
                '(' | ','
                    | '='
                    | '['
                    | '{'
                    | ';'
                    | ':'
                    | '!'
                    | '&'
                    | '|'
                    | '?'
                    | '+'
                    | '-'
                    | '*'
                    | '%'
                    | '^'
                    | '~'
                    | '<'
                    | '>'
                    | '/'
            ) {
                true
            } else if c.is_alphanumeric()
                || c == '_'
                || c == ')'
                || c == ']'
                || c == '}'
                || c == '$'
            {
                // After a value/identifier `/` is division — unless the
                // trailing word is a regex-preceding keyword (`return /…/`).
                ends_with_regex_keyword(out_so_far)
            } else {
                false
            }
        }
    }
}

fn ends_with_regex_keyword(out_so_far: &str) -> bool {
    let trimmed = out_so_far.trim_end();
    let word_start = trimmed
        .rfind(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '$'))
        .map_or(0, |i| i + 1);
    let word = &trimmed[word_start..];
    !word.is_empty() && REGEX_PRECEDING_KEYWORDS.contains(&word)
}

#[allow(clippy::too_many_lines)]
fn mask_line(line: &str, carry: Carry) -> (String, Carry) {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut interp_depth: u32 = 0;
    let mut state = match carry {
        Carry::Code => S::Code,
        Carry::BlockComment => S::BlockComment,
        Carry::Template => S::Template,
        Carry::TemplateInterp(depth) => {
            interp_depth = depth;
            S::TemplateInterp
        }
        Carry::SingleString => S::Single,
        Carry::DoubleString => S::Double,
    };
    let mut regex_in_class = false;
    // Last non-whitespace character emitted as code, for regex/division.
    let mut prev_sig: Option<char> = None;
    // Set when a `\` escape consumes past end-of-line inside a string —
    // signals a line continuation so the string state carries over.
    let mut string_continues = false;

    while let Some(c) = chars.next() {
        match state {
            S::Code => {
                let nxt = chars.peek().copied();
                match c {
                    '/' if nxt == Some('/') => {
                        state = S::LineComment;
                        push_spaces(&mut out, c);
                    }
                    '/' if nxt == Some('*') => {
                        state = S::BlockComment;
                        push_spaces(&mut out, c);
                    }
                    '/' if regex_allowed(prev_sig, &out) => {
                        state = S::Regex;
                        regex_in_class = false;
                        push_spaces(&mut out, c);
                    }
                    '\'' => {
                        state = S::Single;
                        push_spaces(&mut out, c);
                    }
                    '"' => {
                        state = S::Double;
                        push_spaces(&mut out, c);
                    }
                    '`' => {
                        state = S::Template;
                        out.push(c);
                        prev_sig = Some('`');
                    }
                    _ => {
                        out.push(c);
                        if !c.is_whitespace() {
                            prev_sig = Some(c);
                        }
                    }
                }
            }
            S::LineComment => push_spaces(&mut out, c),
            S::BlockComment => {
                push_spaces(&mut out, c);
                // Always mask the current char; if it's the `*` of `*/`,
                // also consume and mask the `/` and return to code.
                if c == '*' && chars.peek() == Some(&'/') {
                    let slash = chars.next().expect("peeked");
                    push_spaces(&mut out, slash);
                    state = S::Code;
                }
            }
            S::Template => {
                // Template literal text is string content — mask it. Keep the
                // structural punctuation (backtick, `${`) so byte offsets and
                // the interpolation boundary stay legible.
                if c == '\\' {
                    push_spaces(&mut out, c);
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    }
                } else if c == '`' {
                    out.push(c);
                    state = S::Code;
                    prev_sig = Some('`');
                } else if c == '$' && chars.peek() == Some(&'{') {
                    let brace = chars.next().expect("peeked");
                    out.push(c);
                    out.push(brace);
                    interp_depth = 1;
                    state = S::TemplateInterp;
                    prev_sig = Some('{');
                } else {
                    push_spaces(&mut out, c);
                }
            }
            S::TemplateInterp => {
                // Real code inside `${ … }` — keep verbatim so the rules scan
                // it. Track brace depth so an inner `{ … }` (object literal,
                // block) does not end the interpolation early.
                out.push(c);
                match c {
                    '{' => interp_depth += 1,
                    '}' => {
                        interp_depth -= 1;
                        if interp_depth == 0 {
                            state = S::Template;
                        }
                    }
                    _ => {}
                }
                if !c.is_whitespace() {
                    prev_sig = Some(c);
                }
            }
            S::Single | S::Double => {
                let closing = if state == S::Single { '\'' } else { '"' };
                push_spaces(&mut out, c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    } else {
                        // `\` at end of line inside a string — continuation.
                        string_continues = true;
                    }
                } else if c == closing {
                    state = S::Code;
                    // A closed string is a value, so a following `/` divides.
                    prev_sig = Some(closing);
                }
            }
            S::Regex => {
                push_spaces(&mut out, c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    }
                } else if c == '[' {
                    regex_in_class = true;
                } else if c == ']' {
                    regex_in_class = false;
                } else if c == '/' && !regex_in_class {
                    state = S::Code;
                    // A regex literal is a value: a following `/` is division,
                    // not a new regex (`/a/ / 2`). Record a value-like token
                    // (`)`) rather than `/` — `/` would re-arm `regex_allowed`
                    // and mis-lex the divisor as a regex, masking real code.
                    prev_sig = Some(')');
                }
            }
        }
    }

    let next_carry = match state {
        S::BlockComment => Carry::BlockComment,
        S::Template => Carry::Template,
        S::TemplateInterp => Carry::TemplateInterp(interp_depth),
        S::Single if string_continues => Carry::SingleString,
        S::Double if string_continues => Carry::DoubleString,
        // LineComment / Regex / unterminated strings do not carry.
        _ => Carry::Code,
    };
    (out, next_carry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_one(line: &str) -> String {
        mask_non_code_lines(&[line]).pop().unwrap()
    }

    #[test]
    fn line_comment_is_masked_after_slashes() {
        let out = mask_one("const x = 1; // keep this! any");
        assert_eq!(out, "const x = 1;                  ");
        assert_eq!(out.len(), "const x = 1; // keep this! any".len());
    }

    #[test]
    fn double_quoted_string_is_masked() {
        let out = mask_one(r#"setSuccess("Account created!");"#);
        assert_eq!(out, "setSuccess(                  );");
        assert!(!out.contains('!'), "bang inside string must be masked");
    }

    #[test]
    fn single_quoted_string_is_masked() {
        let out = mask_one("const s = 'has any keyword';");
        assert!(!out.contains("any"), "got: {out}");
        assert_eq!(out.len(), "const s = 'has any keyword';".len());
    }

    #[test]
    fn escaped_quote_does_not_close_string_early() {
        let out = mask_one(r#"const s = "a\"b! c"; const t: any = x;"#);
        assert!(out.contains(": any"), "got: {out}");
        assert!(!out.contains("b!"), "got: {out}");
    }

    #[test]
    fn code_after_masked_string_keeps_byte_offsets() {
        let line = r#"f("hi!"); const v: any = 1;"#;
        let out = mask_one(line);
        assert_eq!(out.len(), line.len(), "byte length must be preserved");
        assert_eq!(out.find(": any"), line.find(": any"));
    }

    #[test]
    fn block_comment_single_line_is_masked() {
        let out = mask_one("let a = 1; /* drop any! here */ let b: any = 2;");
        assert!(out.contains("let a = 1;"), "got: {out}");
        assert!(out.contains("let b: any = 2;"), "got: {out}");
        assert_eq!(out.matches("any").count(), 1, "got: {out}");
    }

    #[test]
    fn block_comment_spans_lines() {
        let masked =
            mask_non_code_lines(&["a; /* open any!", "still! comment any", "*/ b: any = 1;"]);
        assert!(masked[0].contains("a;"));
        assert!(!masked[0].contains("any"), "line0: {}", masked[0]);
        assert!(!masked[1].contains("any"), "line1: {}", masked[1]);
        assert!(!masked[1].contains('!'), "line1: {}", masked[1]);
        assert!(masked[2].contains("b: any = 1;"), "line2: {}", masked[2]);
    }

    #[test]
    fn template_text_is_masked_but_interpolation_code_survives() {
        let out = mask_one("const s = `hello ${obj!.x} world`;");
        assert!(
            out.contains("obj!.x"),
            "interpolation code (a real non-null assertion) must survive: {out}"
        );
        assert!(
            !out.contains("hello") && !out.contains("world"),
            "template literal text must be masked: {out}"
        );
        assert_eq!(out.len(), "const s = `hello ${obj!.x} world`;".len());
    }

    #[test]
    fn template_text_bang_is_masked() {
        // GS-001 external-FP dogfood (zod `my-0!`, excalidraw `Stack overflow!`):
        // a `!` in template literal *text* is not a non-null assertion.
        let out = mask_one("const c = `my-0! border-none`;");
        assert!(
            !out.contains('!'),
            "template text bang must be masked: {out}"
        );
        assert_eq!(out.len(), "const c = `my-0! border-none`;".len());
    }

    #[test]
    fn template_text_bang_before_interpolation_is_masked() {
        let out = mask_one("err(`Stack overflow! ${size} bytes`);");
        let head = &out[..out.find("${").expect("interp")];
        assert!(
            !head.contains('!'),
            "text bang before interp must mask: {out}"
        );
        assert!(out.contains("size"), "interpolation must survive: {out}");
    }

    #[test]
    fn template_multiline_interpolation_code_survives() {
        let masked =
            mask_non_code_lines(&["const c = `pre ${ map.get(k)", "    .unwrap()! } post`;"]);
        assert!(
            masked[1].contains(".unwrap()!"),
            "multi-line interpolation code must survive: {masked:?}"
        );
    }

    #[test]
    fn template_multiline_text_is_masked() {
        let masked = mask_non_code_lines(&["const c = `line one!", "  line two any`;"]);
        assert!(
            !masked[0].contains('!'),
            "line0 text bang masked: {masked:?}"
        );
        assert!(!masked[1].contains("any"), "line1 text masked: {masked:?}");
    }

    #[test]
    fn quotes_inside_line_comment_do_not_start_a_string() {
        let masked = mask_non_code_lines(&["x = 1; // a \" quote", "const v: any = 2;"]);
        assert!(masked[1].contains(": any"), "got: {}", masked[1]);
    }

    #[test]
    fn plain_code_is_unchanged() {
        let line = "const v: any = compute(a, b);";
        assert_eq!(mask_one(line), line);
    }

    #[test]
    fn multibyte_chars_in_string_preserve_byte_length() {
        let line = r#"const s = "café!"; let v: any = 1;"#;
        let out = mask_one(line);
        assert_eq!(out.len(), line.len(), "byte length must be preserved");
        assert_eq!(out.find(": any"), line.find(": any"));
    }

    // --- Regex literal handling (council adversarial F-1, F-2, F-4, F-5) ---

    #[test]
    fn regex_with_double_slash_does_not_start_line_comment() {
        // `/\/\//` matches a `//`; without regex handling the second `//`
        // would start a line comment and mask the real `: any` after it.
        let line = r"const re = /\/\//; const v: any = 1;";
        let out = mask_one(line);
        assert_eq!(out.find(": any"), line.find(": any"), "got: {out}");
    }

    #[test]
    fn regex_with_quote_does_not_start_string() {
        let line = r#"const re = /["']/; const v: any = 1;"#;
        let out = mask_one(line);
        assert_eq!(out.find(": any"), line.find(": any"), "got: {out}");
    }

    #[test]
    fn regex_bang_in_pattern_is_masked() {
        // GS-001 would otherwise see `user!` inside the pattern.
        let out = mask_one("const re = /user![A-Z]/;");
        assert!(!out.contains("user!"), "regex body must be masked: {out}");
    }

    #[test]
    fn division_is_not_treated_as_regex() {
        // After a value, `/` is division; the rest of the line stays code.
        let line = "const ratio = total / count; const v: any = 1;";
        assert_eq!(mask_one(line), line);
    }

    #[test]
    fn division_after_regex_literal_is_not_treated_as_regex() {
        // `/a/ / count` — the `/` after the regex literal is division, not a
        // new regex. The divisor and the real `: any` tail must survive
        // (regression guard for the post-regex `prev_sig` value-token fix).
        let line = "const r = /a/ / count; const v: any = 1;";
        let out = mask_one(line);
        assert!(out.contains("count"), "divisor must survive: {out}");
        assert_eq!(out.find(": any"), line.find(": any"), "got: {out}");
    }

    #[test]
    fn regex_after_return_keyword_is_masked() {
        let out = mask_one("return /any!/;");
        assert!(!out.contains("any!"), "regex after return must mask: {out}");
    }

    #[test]
    fn url_slashes_inside_string_do_not_start_comment() {
        let line = r#"const u = "http://x.example/a"; const v: any = 1;"#;
        let out = mask_one(line);
        assert_eq!(out.find(": any"), line.find(": any"), "got: {out}");
    }

    #[test]
    fn line_continued_string_carries_and_masks_next_line_tail() {
        // F-3: a `\` at EOL continues the string; the closing quote on the
        // next line must not be read as an opening quote that masks the
        // real `: any` after it.
        let masked =
            mask_non_code_lines(&[r#"const s = "first \"#, r#"second"; const t: any = 1;"#]);
        assert!(!masked[0].contains("first"), "line0: {}", masked[0]);
        assert!(
            masked[1].contains(": any"),
            "real code after continued string must survive: {}",
            masked[1]
        );
        assert!(!masked[1].contains("second"), "line1: {}", masked[1]);
    }
}
