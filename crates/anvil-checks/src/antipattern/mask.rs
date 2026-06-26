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
//! code, are kept and scanned. A carried context stack re-lexes interpolation
//! code, so strings/comments/regexes inside `${ … }` are masked and their braces
//! do not affect interpolation depth. Nested template literals are handled the
//! same way: nested template text is masked, nested `${ … }` code is visible,
//! and control resumes to the correct parent interpolation/template frame.
//!
//! **Regex vs division.** `/` is ambiguous in JS/TS. A `/` is treated as the
//! start of a regex literal only when the preceding significant token implies
//! an expression position (an operator/opening punctuator, statement start,
//! or a regex-preceding keyword such as `return`). After a value (identifier,
//! `)`, `]`, number, string) a `/` is division. This is the same heuristic
//! syntax highlighters use; it is not a full parse, but it covers the common
//! `const re = /…/`, `x.match(/…/)`, and `return /…/` forms.

/// Lexer state that can carry across a line boundary.
#[derive(Clone, PartialEq, Eq)]
struct Carry {
    stack: Vec<Frame>,
}

impl Carry {
    fn code() -> Self {
        Self {
            stack: vec![Frame::Code(CodeCtx {
                prev_sig: None,
                interp_depth: None,
            })],
        }
    }
}

/// Mask the comment and string spans of every line, threading multi-line
/// state (block comments, template literals, line-continued strings) across
/// line boundaries.
pub(crate) fn mask_non_code_lines(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut carry = Carry::code();
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

#[derive(Clone, PartialEq, Eq)]
struct CodeCtx {
    prev_sig: Option<char>,
    /// `Some(depth)` when this code frame is a `${ … }` interpolation.
    interp_depth: Option<u32>,
}

/// Lexer frame within a line. The full stack is carried across lines for
/// multiline comments/templates/interpolations.
#[derive(Clone, PartialEq, Eq)]
enum Frame {
    Code(CodeCtx),
    LineComment,
    BlockComment,
    /// Template literal text (masked).
    Template,
    Single {
        continued: bool,
    },
    Double {
        continued: bool,
    },
    Regex {
        in_class: bool,
    },
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
    let mut stack = carry.stack;
    if stack.is_empty() {
        stack.push(Frame::Code(CodeCtx {
            prev_sig: None,
            interp_depth: None,
        }));
    }

    while let Some(c) = chars.next() {
        let Some(frame) = stack.last_mut() else {
            stack.push(Frame::Code(CodeCtx {
                prev_sig: None,
                interp_depth: None,
            }));
            continue;
        };
        match frame {
            Frame::Code(ctx) => {
                let nxt = chars.peek().copied();
                match c {
                    '/' if nxt == Some('/') => {
                        stack.push(Frame::LineComment);
                        push_spaces(&mut out, c);
                    }
                    '/' if nxt == Some('*') => {
                        stack.push(Frame::BlockComment);
                        push_spaces(&mut out, c);
                    }
                    '/' if regex_allowed(ctx.prev_sig, &out) => {
                        stack.push(Frame::Regex { in_class: false });
                        push_spaces(&mut out, c);
                    }
                    '\'' => {
                        stack.push(Frame::Single { continued: false });
                        push_spaces(&mut out, c);
                    }
                    '"' => {
                        stack.push(Frame::Double { continued: false });
                        push_spaces(&mut out, c);
                    }
                    '`' => {
                        out.push(c);
                        ctx.prev_sig = Some('`');
                        stack.push(Frame::Template);
                    }
                    '{' => {
                        out.push(c);
                        if let Some(depth) = &mut ctx.interp_depth {
                            *depth += 1;
                        }
                        ctx.prev_sig = Some(c);
                    }
                    '}' if ctx.interp_depth.is_some() => {
                        out.push(c);
                        let depth = ctx.interp_depth.as_mut().expect("checked");
                        *depth = depth.saturating_sub(1);
                        if *depth == 0 {
                            stack.pop();
                        } else {
                            ctx.prev_sig = Some(c);
                        }
                    }
                    _ => {
                        out.push(c);
                        if !c.is_whitespace() {
                            ctx.prev_sig = Some(c);
                        }
                    }
                }
            }
            Frame::LineComment => push_spaces(&mut out, c),
            Frame::BlockComment => {
                push_spaces(&mut out, c);
                // Always mask the current char; if it's the `*` of `*/`,
                // also consume and mask the `/` and return to code.
                if c == '*' && chars.peek() == Some(&'/') {
                    let slash = chars.next().expect("peeked");
                    push_spaces(&mut out, slash);
                    stack.pop();
                }
            }
            Frame::Template => {
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
                    stack.pop();
                    if let Some(Frame::Code(ctx)) = stack.last_mut() {
                        ctx.prev_sig = Some('`');
                    }
                } else if c == '$' && chars.peek() == Some(&'{') {
                    let brace = chars.next().expect("peeked");
                    out.push(c);
                    out.push(brace);
                    stack.push(Frame::Code(CodeCtx {
                        prev_sig: Some('{'),
                        interp_depth: Some(1),
                    }));
                } else {
                    push_spaces(&mut out, c);
                }
            }
            Frame::Single { continued } => {
                let closing = '\'';
                push_spaces(&mut out, c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    } else {
                        // `\` at end of line inside a string — continuation.
                        *continued = true;
                    }
                } else if c == closing {
                    stack.pop();
                    // A closed string is a value, so a following `/` divides.
                    if let Some(Frame::Code(ctx)) = stack.last_mut() {
                        ctx.prev_sig = Some(closing);
                    }
                }
            }
            Frame::Double { continued } => {
                let closing = '"';
                push_spaces(&mut out, c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    } else {
                        // `\` at end of line inside a string — continuation.
                        *continued = true;
                    }
                } else if c == closing {
                    stack.pop();
                    // A closed string is a value, so a following `/` divides.
                    if let Some(Frame::Code(ctx)) = stack.last_mut() {
                        ctx.prev_sig = Some(closing);
                    }
                }
            }
            Frame::Regex { in_class } => {
                push_spaces(&mut out, c);
                if c == '\\' {
                    if let Some(n) = chars.next() {
                        push_spaces(&mut out, n);
                    }
                } else if c == '[' {
                    *in_class = true;
                } else if c == ']' {
                    *in_class = false;
                } else if c == '/' && !*in_class {
                    stack.pop();
                    // A regex literal is a value: a following `/` is division,
                    // not a new regex (`/a/ / 2`). Record a value-like token
                    // (`)`) rather than `/` — `/` would re-arm `regex_allowed`
                    // and mis-lex the divisor as a regex, masking real code.
                    if let Some(Frame::Code(ctx)) = stack.last_mut() {
                        ctx.prev_sig = Some(')');
                    }
                }
            }
        }
    }

    normalise_eol_stack(&mut stack);
    if stack.is_empty() {
        stack.push(Frame::Code(CodeCtx {
            prev_sig: None,
            interp_depth: None,
        }));
    }
    (out, Carry { stack })
}

fn normalise_eol_stack(stack: &mut Vec<Frame>) {
    loop {
        let pop = match stack.last_mut() {
            Some(Frame::LineComment | Frame::Regex { .. }) => true,
            Some(Frame::Single { continued } | Frame::Double { continued }) => {
                let carry = *continued;
                *continued = false;
                !carry
            }
            _ => false,
        };
        if pop {
            stack.pop();
        } else {
            break;
        }
    }
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
    fn interpolation_string_masks_tokens_and_braces_without_hiding_real_code() {
        let line = r#"const s = `x ${"} cast as any !" + (cfg as any)}`;"#;
        let out = mask_one(line);
        assert_eq!(out.len(), line.len(), "byte length must be preserved");
        assert!(
            out.contains("cfg as any"),
            "real interpolation code must survive: {out}"
        );
        assert!(
            !out.contains("cast as any") && !out.contains("!\""),
            "interpolation string content must be masked: {out}"
        );
    }

    #[test]
    fn interpolation_block_comment_spans_lines_and_returns_to_interpolation() {
        let masked = mask_non_code_lines(&["const s = `x ${ /* } as any!", "*/ user!.name } y`;"]);
        assert!(
            !masked[0].contains("as any") && !masked[0].contains('!'),
            "comment content must be masked: {masked:?}"
        );
        assert!(
            masked[1].contains("user!.name"),
            "real interpolation code after comment must survive: {masked:?}"
        );
        assert!(
            !masked[1].contains(" y"),
            "template text after interpolation close must be masked: {masked:?}"
        );
    }

    #[test]
    fn nested_template_masks_text_keeps_nested_and_outer_interpolation_code() {
        let line = "const s = `outer ${cond ? `inner any! ${user!.id}` : fallback!.id} tail`;";
        let out = mask_one(line);
        assert!(
            !out.contains("inner any"),
            "nested template text must be masked: {out}"
        );
        assert!(
            out.contains("user!.id"),
            "nested interpolation code must survive: {out}"
        );
        assert!(
            out.contains("fallback!.id"),
            "outer interpolation code after nested template must survive: {out}"
        );
        assert!(
            !out.contains(" tail"),
            "outer template tail must be masked: {out}"
        );
    }

    #[test]
    fn escaped_template_delimiters_stay_template_text() {
        let line = r"const s = `\${notCode as any} ${real as any}`;";
        let out = mask_one(line);
        assert!(
            !out.contains("notCode as any"),
            "escaped interpolation opener must remain masked text: {out}"
        );
        assert!(
            out.contains("real as any"),
            "real interpolation code must survive: {out}"
        );
    }

    #[test]
    fn regex_and_division_inside_interpolation_keep_depth_and_code_visibility() {
        let regex = "const s = `x ${/[}!]/.test(v) ? cfg as any : fallback}`;";
        let regex_out = mask_one(regex);
        assert!(
            !regex_out.contains("}!"),
            "regex body inside interpolation must be masked: {regex_out}"
        );
        assert!(
            regex_out.contains("cfg as any"),
            "code after interpolation regex must survive: {regex_out}"
        );

        let div = "const s = `x ${total / count ? cfg as any : fallback}`;";
        let div_out = mask_one(div);
        assert!(
            div_out.contains("total / count"),
            "division survives: {div_out}"
        );
        assert!(div_out.contains("cfg as any"), "code survives: {div_out}");
    }

    #[test]
    fn carried_interpolation_preserves_line_comment_and_division_context() {
        let comment = mask_non_code_lines(&["const s = `x ${ // } as any!", "cfg as any}`;"]);
        assert!(
            !comment[0].contains("as any") && !comment[0].contains('!'),
            "line comment inside interpolation should mask to EOL: {comment:?}"
        );
        assert!(
            comment[1].contains("cfg as any"),
            "next line must resume interpolation code: {comment:?}"
        );

        let division = mask_non_code_lines(&["const s = `x ${total", "  / count; cfg as any}`;"]);
        assert!(
            division[1].contains("/ count; cfg as any"),
            "division and later code must survive carried interpolation: {division:?}"
        );
    }

    #[test]
    fn multibyte_interpolation_string_preserves_later_code_offset() {
        let line = r#"const s = `x ${"café } !"} ${cfg as any}`;"#;
        let out = mask_one(line);
        assert_eq!(out.len(), line.len(), "byte length must be preserved");
        assert_eq!(out.find("as any"), line.find("as any"));
        assert!(!out.contains("café"), "multibyte string text masked: {out}");
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
