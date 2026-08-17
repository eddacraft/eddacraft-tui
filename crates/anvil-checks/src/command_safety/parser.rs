use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::command_safety::types::ParsedCommand;

const MAX_UNWRAP_DEPTH: usize = 5;

const SHELL_WRAPPERS: &[&str] = &["bash", "sh", "zsh", "dash", "ash"];
const PRIVILEGED_WRAPPERS: &[&str] = &["sudo", "doas"];
const ENV_WRAPPERS: &[&str] = &[
    "env", "command", "builtin", "nohup", "nice", "time", "strace",
];
const INTERPRETER_COMMANDS: &[&str] = &["python", "python3", "node", "ruby", "perl", "php"];
const SHELL_LIKE_INTERPRETERS: &[&str] = &["bash", "sh", "zsh", "dash", "ash"];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShellToken {
    Word(String),
    Operator(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubCommandTokens {
    tokens: Vec<String>,
    operator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenisedWithOperators {
    tokens: Vec<String>,
    is_compound: bool,
    sub_commands: Vec<SubCommandTokens>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnwrapResult {
    unwrapped: String,
    wrappers: Vec<String>,
    incomplete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompoundCommandResult {
    #[serde(rename = "isCompound")]
    pub is_compound: bool,
    pub commands: Vec<ParsedCommand>,
    pub operators: Vec<String>,
    #[serde(skip)]
    pub(crate) raw: String,
}

#[must_use]
fn is_shell_wrapper(cmd: &str) -> bool {
    SHELL_WRAPPERS.contains(&normalise_command_name(cmd).as_str())
}

#[must_use]
fn is_privileged_wrapper(cmd: &str) -> bool {
    PRIVILEGED_WRAPPERS.contains(&normalise_command_name(cmd).as_str())
}

#[must_use]
fn is_env_wrapper(cmd: &str) -> bool {
    ENV_WRAPPERS.contains(&normalise_command_name(cmd).as_str())
}

#[must_use]
fn is_interpreter(cmd: &str) -> bool {
    INTERPRETER_COMMANDS.contains(&normalise_command_name(cmd).as_str())
}

/// Reduce an executable token to its basename for wrapper recognition and rule
/// matching. Path forms such as `/bin/rm` and `./rm` both become `rm`. Bare
/// names are left unchanged. Degenerate path-only tokens (e.g. `/`) are kept.
#[must_use]
fn normalise_command_name(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if !trimmed.contains('/') && !trimmed.contains('\\') {
        return trimmed.to_string();
    }
    let stripped = trimmed.trim_end_matches(['/', '\\']);
    if stripped.is_empty() {
        return trimmed.to_string();
    }
    stripped
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(stripped)
        .to_string()
}

#[must_use]
pub(crate) fn shell_code_before_comment(line: &str) -> &str {
    shell_code_before_comment_with_state(line, &mut ShellCommentState::default())
}

#[derive(Default, PartialEq, Eq)]
enum ShellQuoteState {
    #[default]
    None,
    Single,
    Double,
}

#[derive(Default)]
struct ShellCommentFrame {
    quote: ShellQuoteState,
    escaped: bool,
    word_position: ShellWordPosition,
    word_is_command: bool,
    closing: Option<char>,
    paren_depth: usize,
    case_stack: Vec<ShellCasePhase>,
    last_was_semicolon: bool,
    word: String,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum ShellWordPosition {
    #[default]
    Within,
    Argument,
    Command,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShellCasePhase {
    AwaitingIn,
    Pattern,
    Body,
}

pub(crate) struct ShellCommentState {
    frames: Vec<ShellCommentFrame>,
}

impl Default for ShellCommentState {
    fn default() -> Self {
        Self {
            frames: vec![ShellCommentFrame {
                word_position: ShellWordPosition::Command,
                ..ShellCommentFrame::default()
            }],
        }
    }
}

#[must_use]
pub(crate) fn shell_code_before_comment_with_state<'a>(
    line: &'a str,
    state: &mut ShellCommentState,
) -> &'a str {
    if state
        .frames
        .last()
        .is_some_and(|frame| frame.quote == ShellQuoteState::None)
    {
        let frame = state.frames.last_mut().expect("root frame");
        if frame.word_position == ShellWordPosition::Within {
            frame.word_position = ShellWordPosition::Argument;
        }
    }

    let mut characters = line.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        let frame = state.frames.last_mut().expect("root frame");
        if frame.escaped {
            frame.escaped = false;
            frame.word_position = ShellWordPosition::Within;
            continue;
        }
        if character == '\\' && frame.quote != ShellQuoteState::Single {
            frame.escaped = true;
            frame.word_position = ShellWordPosition::Within;
            continue;
        }
        if character == '$'
            && frame.quote != ShellQuoteState::Single
            && characters.peek().is_some_and(|(_, next)| *next == '(')
        {
            frame.word_position = ShellWordPosition::Within;
            let _ = characters.next();
            state.frames.push(fresh_shell_comment_frame(')'));
            continue;
        }
        if character == '`' && frame.quote != ShellQuoteState::Single {
            if frame.closing == Some('`') && frame.quote == ShellQuoteState::None {
                let _ = state.frames.pop();
            } else {
                frame.word_position = ShellWordPosition::Within;
                state.frames.push(fresh_shell_comment_frame('`'));
            }
            continue;
        }
        if frame.quote == ShellQuoteState::None
            && (character == '_' || character.is_ascii_alphanumeric())
        {
            if frame.word.is_empty() {
                frame.word_is_command = frame.word_position == ShellWordPosition::Command;
            }
            frame.word.push(character);
            frame.word_position = ShellWordPosition::Within;
            frame.last_was_semicolon = false;
            continue;
        }
        if character == '#'
            && frame.quote == ShellQuoteState::None
            && frame.word.is_empty()
            && frame.word_position != ShellWordPosition::Within
        {
            finish_shell_comment_line(frame);
            return &line[..index];
        }
        finish_shell_comment_word(frame, Some(character));
        match consume_shell_comment_delimiter(frame, character) {
            ShellCommentAction::Pop => {
                let _ = state.frames.pop();
            }
            ShellCommentAction::Continue => {}
        }
    }
    if let Some(frame) = state.frames.last_mut() {
        finish_shell_comment_line(frame);
    }
    line
}

fn finish_shell_comment_line(frame: &mut ShellCommentFrame) {
    finish_shell_comment_word(frame, None);
    let continued = frame.escaped;
    frame.escaped = false;
    if !continued
        && frame.quote == ShellQuoteState::None
        && frame.case_stack.last() != Some(&ShellCasePhase::AwaitingIn)
        && frame.case_stack.last() != Some(&ShellCasePhase::Pattern)
    {
        frame.word_position = ShellWordPosition::Command;
    }
}

enum ShellCommentAction {
    Continue,
    Pop,
}

fn consume_shell_comment_delimiter(
    frame: &mut ShellCommentFrame,
    character: char,
) -> ShellCommentAction {
    match character {
        '\'' if frame.quote != ShellQuoteState::Double => {
            frame.quote = if frame.quote == ShellQuoteState::Single {
                ShellQuoteState::None
            } else {
                ShellQuoteState::Single
            };
            frame.word_position = ShellWordPosition::Within;
        }
        '"' if frame.quote != ShellQuoteState::Single => {
            frame.quote = if frame.quote == ShellQuoteState::Double {
                ShellQuoteState::None
            } else {
                ShellQuoteState::Double
            };
            frame.word_position = ShellWordPosition::Within;
        }
        '(' if frame.quote == ShellQuoteState::None => frame.paren_depth += 1,
        ')' if frame.quote == ShellQuoteState::None && frame.paren_depth > 0 => {
            frame.paren_depth -= 1;
        }
        ')' if frame.quote == ShellQuoteState::None
            && frame.closing == Some(')')
            && frame.case_stack.is_empty() =>
        {
            return ShellCommentAction::Pop;
        }
        ')' if frame.quote == ShellQuoteState::None
            && frame.case_stack.last() == Some(&ShellCasePhase::Pattern) =>
        {
            *frame.case_stack.last_mut().expect("case phase") = ShellCasePhase::Body;
            frame.word_position = ShellWordPosition::Command;
        }
        ';' if frame.quote == ShellQuoteState::None => {
            if frame.last_was_semicolon && frame.case_stack.last() == Some(&ShellCasePhase::Body) {
                *frame.case_stack.last_mut().expect("case phase") = ShellCasePhase::Pattern;
                frame.word_position = ShellWordPosition::Argument;
            } else {
                frame.word_position = ShellWordPosition::Command;
            }
            frame.last_was_semicolon = true;
        }
        character
            if frame.quote == ShellQuoteState::None
                && matches!(character, '|' | '&' | '(' | ')') =>
        {
            frame.word_position = ShellWordPosition::Command;
            frame.last_was_semicolon = false;
        }
        character if frame.quote == ShellQuoteState::None && character.is_whitespace() => {
            if frame.word_position == ShellWordPosition::Within {
                frame.word_position = ShellWordPosition::Argument;
            }
        }
        _ => {
            frame.word_position = ShellWordPosition::Within;
            frame.last_was_semicolon = false;
        }
    }
    ShellCommentAction::Continue
}

fn fresh_shell_comment_frame(closing: char) -> ShellCommentFrame {
    ShellCommentFrame {
        closing: Some(closing),
        word_position: ShellWordPosition::Command,
        ..ShellCommentFrame::default()
    }
}

fn finish_shell_comment_word(frame: &mut ShellCommentFrame, delimiter: Option<char>) {
    if frame.word.is_empty() {
        return;
    }
    match frame.case_stack.last().copied() {
        Some(ShellCasePhase::AwaitingIn) => {
            if frame.word == "in" {
                *frame.case_stack.last_mut().expect("case phase") = ShellCasePhase::Pattern;
            }
            frame.word_position = ShellWordPosition::Argument;
        }
        Some(ShellCasePhase::Pattern) => {
            if frame.word == "esac" && delimiter != Some(')') {
                let _ = frame.case_stack.pop();
            }
            frame.word_position = ShellWordPosition::Argument;
        }
        Some(ShellCasePhase::Body) | None if frame.word_is_command => match frame.word.as_str() {
            "case" => {
                frame.case_stack.push(ShellCasePhase::AwaitingIn);
                frame.word_position = ShellWordPosition::Argument;
            }
            "esac" if !frame.case_stack.is_empty() => {
                let _ = frame.case_stack.pop();
                frame.word_position = ShellWordPosition::Argument;
            }
            "if" | "while" | "until" | "elif" | "then" | "do" | "else" => {
                frame.word_position = ShellWordPosition::Command;
            }
            _ => frame.word_position = ShellWordPosition::Argument,
        },
        Some(ShellCasePhase::Body) | None => frame.word_position = ShellWordPosition::Argument,
    }
    frame.word.clear();
    frame.word_is_command = false;
}

#[must_use]
pub(crate) fn ends_with_open_pipe(line: &str) -> bool {
    let code = shell_code_before_comment(line).trim_end();
    let pipe_index = if code.ends_with("|&") {
        code.len().saturating_sub(2)
    } else if code.ends_with('|') && !code.ends_with("||") {
        code.len().saturating_sub(1)
    } else {
        return false;
    };

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    for (index, character) in code.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '|' if index == pipe_index => return !in_single_quote && !in_double_quote,
            _ => {}
        }
    }
    false
}

#[must_use]
pub(crate) fn starts_with_pipe(line: &str) -> bool {
    let code = shell_code_before_comment(line).trim_start();
    code.starts_with('|') && !code.starts_with("||")
}

/// Whether a physical line ends inside shell syntax that must be completed by
/// a later line. This intentionally tracks only syntax that affects command
/// boundaries: quotes, parenthesised substitutions/groups, and brace groups.
#[must_use]
pub(crate) fn shell_construct_is_open(text: &str) -> bool {
    #[derive(Default)]
    struct Frame {
        closing: Option<char>,
        in_single_quote: bool,
        in_double_quote: bool,
        escaped: bool,
    }

    let chars = text.chars().collect::<Vec<_>>();
    let mut frames = vec![Frame::default()];
    let mut index = 0usize;
    while index < chars.len() {
        let character = chars[index];
        let frame = frames.last_mut().expect("root shell frame");
        if frame.escaped {
            frame.escaped = false;
            index += 1;
            continue;
        }
        if character == '\\' && !frame.in_single_quote {
            frame.escaped = true;
            index += 1;
            continue;
        }
        match character {
            '\'' if !frame.in_double_quote => {
                frame.in_single_quote = !frame.in_single_quote;
                index += 1;
                continue;
            }
            '"' if !frame.in_single_quote => {
                frame.in_double_quote = !frame.in_double_quote;
                index += 1;
                continue;
            }
            _ => {}
        }
        if frame.in_single_quote {
            index += 1;
            continue;
        }
        let substitution = matches!(character, '$' | '<') && chars.get(index + 1) == Some(&'(');
        let parameter = character == '$' && chars.get(index + 1) == Some(&'{');
        if substitution || parameter {
            frames.push(Frame {
                closing: Some(if parameter { '}' } else { ')' }),
                ..Frame::default()
            });
            index += 2;
            continue;
        }
        let frame = frames.last().expect("root shell frame");
        if !frame.in_double_quote && frame.closing == Some(character) {
            let _ = frames.pop();
            index += 1;
            continue;
        }
        let brace_group = character == '{' && brace_group_opens_here(&chars, index);
        if !frame.in_double_quote && (character == '(' || brace_group) {
            frames.push(Frame {
                closing: Some(if character == '(' { ')' } else { '}' }),
                ..Frame::default()
            });
        }
        index += 1;
    }

    frames.len() > 1
        || frames
            .first()
            .is_some_and(|frame| frame.in_single_quote || frame.in_double_quote)
        || case_construct_is_open(text)
}

fn brace_group_opens_here(chars: &[char], index: usize) -> bool {
    if chars
        .get(index + 1)
        .is_some_and(|character| !character.is_whitespace())
    {
        return false;
    }
    let prefix = chars[..index].iter().collect::<String>();
    let prefix = prefix.trim_end();
    if prefix.is_empty()
        || prefix
            .chars()
            .next_back()
            .is_some_and(|character| matches!(character, ';' | '|' | '&' | '(' | ')' | '!'))
    {
        return true;
    }
    is_function_declaration_prefix(prefix)
        || prefix
            .rsplit(|character: char| {
                character.is_whitespace() || matches!(character, ';' | '|' | '&')
            })
            .next()
            .is_some_and(|word| matches!(word, "then" | "do" | "else"))
}

fn is_function_declaration_prefix(prefix: &str) -> bool {
    let trimmed = prefix.trim();
    if let Some(name) = trimmed.strip_suffix("()") {
        return is_shell_identifier(name.trim());
    }
    let Some(rest) = trimmed.strip_prefix("function ") else {
        return false;
    };
    let name = rest.trim().strip_suffix("()").unwrap_or(rest.trim());
    is_shell_identifier(name)
}

fn is_shell_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn case_construct_is_open(text: &str) -> bool {
    case_reserved_word_balance(text) > 0
}

fn case_reserved_word_balance(text: &str) -> isize {
    let masked = mask_quoted_shell_data(text);
    let bytes = masked.as_bytes();
    let mut balance = 0isize;
    let mut command_position = true;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' || matches!(byte, b';' | b'|' | b'&' | b'(' | b'{' | b')') {
            command_position = true;
            index += 1;
            continue;
        }
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'_' || byte.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let word = &masked[start..index];
            let next = bytes[index..]
                .iter()
                .copied()
                .find(|candidate| !candidate.is_ascii_whitespace());
            if command_position && next != Some(b')') {
                if word == "case" {
                    balance += 1;
                } else if word == "esac" {
                    balance -= 1;
                }
            }
            command_position = matches!(word, "then" | "do" | "else" | "elif");
            continue;
        }
        command_position = false;
        index += 1;
    }
    balance
}

fn mask_quoted_shell_data(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            for _ in 0..character.len_utf8() {
                masked.push(' ');
            }
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            masked.push(' ');
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                masked.push(' ');
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                masked.push(' ');
            }
            _ if in_single_quote || in_double_quote => {
                for _ in 0..character.len_utf8() {
                    masked.push(' ');
                }
            }
            _ => masked.push(character),
        }
    }
    masked
}

const PIPELINE_GRAMMAR_PREFIXES: &[&str] = &[
    "if", "then", "else", "elif", "fi", "while", "until", "do", "done", "!",
];

#[must_use]
fn pipeline_wrapper_option_takes_value(wrapper: &str, flag: &str) -> bool {
    match wrapper {
        "sudo" | "doas" => matches!(
            flag,
            "-u" | "-g"
                | "-C"
                | "-h"
                | "-p"
                | "-r"
                | "-t"
                | "-T"
                | "-U"
                | "-D"
                | "-a"
                | "--user"
                | "--group"
                | "--close-from"
                | "--host"
                | "--prompt"
                | "--role"
                | "--type"
                | "--command-timeout"
                | "--other-user"
                | "--chdir"
                | "--login-class"
        ),
        "env" => matches!(flag, "-C" | "--chdir" | "-u" | "--unset" | "-a" | "--argv0"),
        "nice" => matches!(flag, "-n" | "--adjustment"),
        "time" => matches!(flag, "-f" | "--format" | "-o" | "--output"),
        "strace" => matches!(
            flag,
            "-o" | "--output"
                | "-e"
                | "--trace"
                | "-p"
                | "--attach"
                | "-u"
                | "--user"
                | "-s"
                | "--string-limit"
        ),
        "timeout" => matches!(flag, "-s" | "--signal" | "-k" | "--kill-after"),
        "exec" => flag == "-a",
        _ => false,
    }
}

#[must_use]
fn skip_pipeline_wrapper(tokens: &[String], index: usize, wrapper: &str) -> usize {
    let mut next = index + 1;
    while next < tokens.len() {
        let token = &tokens[next];
        if token == "--" {
            return next + 1;
        }
        if wrapper == "env" && is_environment_assignment(token) {
            next += 1;
            continue;
        }
        if !token.starts_with('-') {
            break;
        }
        let has_inline_value = token.starts_with("--") && token.contains('=');
        if pipeline_wrapper_option_takes_value(wrapper, token) && !has_inline_value {
            next = (next + 2).min(tokens.len());
        } else {
            next += 1;
        }
    }
    next
}

/// First executable in a pipeline stage after skipping grammar prefixes and
/// sudo/env/timeout/busybox, but **not** peeling `sh`/`bash -c`.
#[must_use]
pub(crate) fn pipeline_stage_parts(raw: &str) -> (String, Vec<String>) {
    let tokens = tokenise(raw);
    let mut index = skip_command_prefixes(&tokens, 0);
    while index < tokens.len() {
        let after_prefixes = skip_command_prefixes(&tokens, index);
        if after_prefixes != index {
            index = after_prefixes;
            continue;
        }
        let token = &tokens[index];
        if is_environment_assignment(token) {
            index += 1;
            continue;
        }
        let name = normalise_command_name(token);
        let name_l = name.to_ascii_lowercase();
        if PIPELINE_GRAMMAR_PREFIXES.contains(&name_l.as_str()) {
            index += 1;
            continue;
        }
        if is_privileged_wrapper(&name) || is_env_wrapper(&name) || name_l == "exec" {
            index = skip_pipeline_wrapper(&tokens, index, &name_l);
            continue;
        }
        if name_l == "timeout" {
            index = skip_pipeline_wrapper(&tokens, index, &name_l);
            // timeout requires one duration operand before its command.
            if index < tokens.len() {
                index += 1;
            }
            continue;
        }
        if name_l == "busybox" && index + 1 < tokens.len() {
            let applet = skip_command_prefixes(&tokens, index + 1);
            if applet >= tokens.len() {
                return (String::new(), Vec::new());
            }
            return (
                normalise_command_name(&tokens[applet]),
                tokens[applet + 1..].to_vec(),
            );
        }
        return (name, tokens[index + 1..].to_vec());
    }
    (String::new(), Vec::new())
}

#[must_use]
pub(crate) fn redirection_shape(token: &str) -> Option<bool> {
    let mut without_fd = token.trim_start_matches(|character: char| character.is_ascii_digit());
    if let Some(rest) = without_fd.strip_prefix('{')
        && let Some(close) = rest.find('}')
    {
        without_fd = &rest[close + 1..];
    }
    for operator in [
        "<<<", "<<-", "<<", "&>>", "&>", ">>", ">|", "<>", ">&", "<&", ">", "<",
    ] {
        if let Some(target) = without_fd.strip_prefix(operator) {
            return Some(!target.is_empty());
        }
    }
    None
}

#[must_use]
fn skip_command_prefixes(tokens: &[String], mut index: usize) -> usize {
    loop {
        while index < tokens.len() && is_environment_assignment(&tokens[index]) {
            index += 1;
        }
        let Some(token) = tokens.get(index) else {
            return index;
        };
        let Some(has_inline_target) = redirection_shape(token) else {
            return index;
        };
        index += 1;
        if !has_inline_target && index < tokens.len() {
            index += 1;
        }
    }
}

/// First executable in a pipeline stage after skipping grammar prefixes and
/// supported wrappers, while retaining shell executables such as `bash -c`.
#[must_use]
pub fn pipeline_stage_head(raw: &str) -> String {
    pipeline_stage_parts(raw).0
}

#[must_use]
fn is_environment_assignment(token: &str) -> bool {
    let Some((name, _value)) = token.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if first != '_' && !first.is_ascii_alphabetic() {
        return false;
    }
    characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[must_use]
fn split_by_operators(tokens: &[ShellToken]) -> Vec<SubCommandTokens> {
    let mut commands = Vec::new();
    let mut current_tokens: Vec<String> = Vec::new();
    let mut last_operator: Option<String> = None;

    for token in tokens {
        match token {
            ShellToken::Operator(op) => {
                if !current_tokens.is_empty() {
                    commands.push(SubCommandTokens {
                        tokens: current_tokens,
                        operator: last_operator,
                    });
                    current_tokens = Vec::new();
                }
                last_operator = Some(op.clone());
            }
            ShellToken::Word(word) => current_tokens.push(word.clone()),
        }
    }

    if !current_tokens.is_empty() {
        commands.push(SubCommandTokens {
            tokens: current_tokens,
            operator: last_operator,
        });
    }

    commands
}

fn begin_single_quote(current: &mut String) -> bool {
    let preserve = current.ends_with('$');
    if preserve {
        current.push('\'');
    }
    preserve
}

fn close_single_quote(current: &mut String, preserve: &mut bool) {
    if *preserve {
        current.push('\'');
        *preserve = false;
    }
}

fn push_current_word(tokens: &mut Vec<ShellToken>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(ShellToken::Word(std::mem::take(current)));
    }
}

#[derive(Default)]
struct SubstitutionState {
    depth: usize,
    in_single_quote: bool,
    in_double_quote: bool,
    escaped: bool,
}

impl SubstitutionState {
    fn begin(&mut self) {
        self.depth = 1;
    }

    fn consume(&mut self, character: char) {
        if self.escaped {
            self.escaped = false;
            return;
        }
        if character == '\\' && !self.in_single_quote {
            self.escaped = true;
            return;
        }
        match character {
            '\'' if !self.in_double_quote => self.in_single_quote = !self.in_single_quote,
            '"' if !self.in_single_quote => self.in_double_quote = !self.in_double_quote,
            '(' if !self.in_single_quote && !self.in_double_quote => self.depth += 1,
            ')' if !self.in_single_quote && !self.in_double_quote => {
                self.depth = self.depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn active(&self) -> bool {
        self.depth > 0
    }
}

fn consume_single_quoted(
    character: char,
    in_single_quote: &mut bool,
    current: &mut String,
    preserve_single_quote: &mut bool,
) -> bool {
    if !*in_single_quote {
        return false;
    }
    if character == '\'' {
        *in_single_quote = false;
        close_single_quote(current, preserve_single_quote);
    } else {
        current.push(character);
    }
    true
}

fn consume_double_quoted(
    character: char,
    in_double_quote: &mut bool,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    current: &mut String,
) -> bool {
    if !*in_double_quote {
        return false;
    }
    if character == '\\' {
        if let Some(next) = chars.next() {
            if matches!(next, '$' | '`' | '"' | '\\' | '\n') {
                current.push(next);
            } else {
                current.push('\\');
                current.push(next);
            }
        }
    } else if character == '"' {
        *in_double_quote = false;
    } else {
        current.push(character);
    }
    true
}

#[must_use]
fn tokenise_shell(cmd: &str) -> Vec<ShellToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = cmd.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut preserve_single_quote = false;
    let mut substitution = SubstitutionState::default();
    let mut trailing_gt_is_operator = false;

    while let Some(ch) = chars.next() {
        if consume_single_quoted(
            ch,
            &mut in_single_quote,
            &mut current,
            &mut preserve_single_quote,
        ) {
            trailing_gt_is_operator = false;
            continue;
        }

        if consume_double_quoted(ch, &mut in_double_quote, &mut chars, &mut current) {
            trailing_gt_is_operator = false;
            continue;
        }

        if substitution.active() {
            current.push(ch);
            substitution.consume(ch);
            trailing_gt_is_operator = false;
            continue;
        }
        if matches!(ch, '$' | '<' | '>') && chars.peek() == Some(&'(') {
            current.push(ch);
            current.push(chars.next().expect("peeked opening parenthesis"));
            substitution.begin();
            trailing_gt_is_operator = false;
            continue;
        }

        match ch {
            '\'' => {
                preserve_single_quote = begin_single_quote(&mut current);
                in_single_quote = true;
            }
            '"' => in_double_quote = true,
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
                trailing_gt_is_operator = false;
            }
            '#' => {
                if current.is_empty() {
                    break;
                }
                current.push('#');
            }
            '&' => {
                // `2>&1` / `>&1` and `2<&1` / `<&1` are redirections, not
                // background separators.
                if trailing_gt_is_operator || current.ends_with('<') {
                    current.push('&');
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                    trailing_gt_is_operator = false;
                    continue;
                }
                if chars.peek() == Some(&'>') {
                    current.push('&');
                    current.push(chars.next().expect("peeked >"));
                    trailing_gt_is_operator = true;
                    continue;
                }
                push_current_word(&mut tokens, &mut current);
                if chars.peek() == Some(&'&') {
                    let _ = chars.next();
                    tokens.push(ShellToken::Operator("&&".to_string()));
                } else {
                    tokens.push(ShellToken::Operator("&".to_string()));
                }
                trailing_gt_is_operator = false;
            }
            '|' => consume_pipe_operator(
                &mut chars,
                &mut tokens,
                &mut current,
                &mut trailing_gt_is_operator,
            ),
            ';' | '\n' => {
                push_current_word(&mut tokens, &mut current);
                tokens.push(ShellToken::Operator(";".to_string()));
                trailing_gt_is_operator = false;
            }
            c if c.is_whitespace() => {
                push_current_word(&mut tokens, &mut current);
                trailing_gt_is_operator = false;
            }
            _ => {
                current.push(ch);
                trailing_gt_is_operator = ch == '>';
            }
        }
    }

    push_current_word(&mut tokens, &mut current);
    tokens
}

fn consume_pipe_operator(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    tokens: &mut Vec<ShellToken>,
    current: &mut String,
    trailing_gt_is_operator: &mut bool,
) {
    if *trailing_gt_is_operator {
        current.push('|');
        *trailing_gt_is_operator = false;
        return;
    }
    push_current_word(tokens, current);
    if chars.peek() == Some(&'|') {
        let _ = chars.next();
        tokens.push(ShellToken::Operator("||".to_string()));
    } else {
        if chars.peek() == Some(&'&') {
            // bash `|&` is a pipe of stdout+stderr, not a background `&`.
            let _ = chars.next();
        }
        tokens.push(ShellToken::Operator("|".to_string()));
    }
    *trailing_gt_is_operator = false;
}

#[must_use]
fn tokenise(cmd: &str) -> Vec<String> {
    tokenise_shell(cmd)
        .into_iter()
        .filter_map(|token| match token {
            ShellToken::Word(word) if !word.is_empty() => Some(word),
            ShellToken::Word(_) | ShellToken::Operator(_) => None,
        })
        .collect()
}

#[must_use]
pub(crate) fn shell_words(cmd: &str) -> Vec<String> {
    tokenise(cmd)
}

#[must_use]
pub(crate) fn exec_descriptor_update(text: &str) -> bool {
    parse_compound_command(text)
        .commands
        .iter()
        .any(|command| !persistent_exec_descriptor_updates(&command.raw).is_empty())
}

#[must_use]
pub(crate) fn persistent_exec_descriptor_updates(
    text: &str,
) -> Vec<PersistentExecDescriptorUpdate> {
    collect_persistent_exec_descriptor_updates(text, false)
}

pub(crate) struct PersistentExecDescriptorUpdate {
    pub(crate) words: Vec<String>,
    pub(crate) conditional: bool,
}

fn collect_persistent_exec_descriptor_updates(
    text: &str,
    inherited_conditional: bool,
) -> Vec<PersistentExecDescriptorUpdate> {
    let compound = parse_compound_command(text);
    let mut updates = Vec::new();
    let mut conditional = inherited_conditional;
    for (index, command) in compound.commands.iter().enumerate() {
        if index > 0 {
            match compound.operators.get(index - 1).map(String::as_str) {
                Some(";") => conditional = inherited_conditional,
                Some("&&" | "||") => conditional = true,
                _ => return Vec::new(),
            }
        }

        let words = shell_words(&command.raw);
        if descriptor_only_exec_words(&words) {
            updates.push(PersistentExecDescriptorUpdate { words, conditional });
            continue;
        }

        let (head, args) = pipeline_stage_parts(&command.raw);
        if !head.eq_ignore_ascii_case("eval") {
            continue;
        }
        let args = args.strip_prefix(&["--".to_string()]).unwrap_or(&args);
        let payload = args.join(" ");
        if !payload.is_empty() && payload.len() < command.raw.len() {
            updates.extend(collect_persistent_exec_descriptor_updates(
                &payload,
                conditional,
            ));
        }
    }
    updates
}

fn descriptor_only_exec_words(words: &[String]) -> bool {
    let Some(exec_index) = exec_command_index(words) else {
        return false;
    };
    let mut index = exec_index + 1;
    let mut saw_redirection = words[..exec_index]
        .iter()
        .any(|word| redirection_shape(word).is_some());
    while index < words.len() {
        let Some(has_inline_target) = redirection_shape(&words[index]) else {
            return false;
        };
        saw_redirection = true;
        index += if has_inline_target { 1 } else { 2 };
    }
    saw_redirection && index == words.len()
}

#[must_use]
pub(crate) fn exec_command_index(words: &[String]) -> Option<usize> {
    let mut index = 0usize;
    while index < words.len() {
        let word = &words[index];
        if word == "exec" {
            return Some(index);
        }
        if matches!(word.as_str(), "{" | "!" | "then" | "do") || is_environment_assignment(word) {
            index += 1;
            continue;
        }
        if let Some(has_inline_target) = redirection_shape(word) {
            index += if has_inline_target { 1 } else { 2 };
            continue;
        }
        return None;
    }
    None
}

#[must_use]
fn tokenise_with_operators(cmd: &str) -> TokenisedWithOperators {
    let parsed = tokenise_shell(cmd);
    let has_operator = parsed
        .iter()
        .any(|token| matches!(token, ShellToken::Operator(_)));

    if !has_operator {
        let tokens = parsed
            .into_iter()
            .filter_map(|token| match token {
                ShellToken::Word(word) if !word.is_empty() => Some(word),
                ShellToken::Word(_) | ShellToken::Operator(_) => None,
            })
            .collect::<Vec<_>>();

        return TokenisedWithOperators {
            tokens: tokens.clone(),
            is_compound: false,
            sub_commands: vec![SubCommandTokens {
                tokens,
                operator: None,
            }],
        };
    }

    let sub_commands = split_by_operators(&parsed);
    let all_tokens = sub_commands
        .iter()
        .flat_map(|sub| sub.tokens.clone())
        .collect::<Vec<_>>();

    TokenisedWithOperators {
        tokens: all_tokens,
        is_compound: true,
        sub_commands,
    }
}

#[must_use]
pub(crate) fn shell_option_invokes_command(token: &str) -> bool {
    token.starts_with('-')
        && !token.starts_with("--")
        && token.chars().skip(1).any(|character| character == 'c')
}

#[must_use]
fn extract_shell_wrapper_arg(tokens: &[String]) -> Option<String> {
    for (index, token) in tokens.iter().enumerate() {
        if shell_option_invokes_command(token) {
            let mut payload = index + 1;
            if tokens.get(payload).is_some_and(|token| token == "--") {
                payload += 1;
            }
            return tokens.get(payload).cloned();
        }
    }
    None
}

#[must_use]
fn extract_env_command(tokens: &[String]) -> Option<Vec<String>> {
    const ENV_OPTIONS_WITH_VALUE: &[&str] = &["-C", "--chdir", "-u", "--unset", "-a", "--argv0"];

    let mut start_index = 1;
    while start_index < tokens.len() {
        let token = &tokens[start_index];
        // GNU `env -S` / `--split-string` takes a command *line*, not a
        // skippable option value. Treating it like `-C` hid `env -S "rm -rf /"`
        // as a bare `env`. Re-tokenise the payload the same way `bash -c` is
        // unwrapped.
        if token == "-S" || token == "--split-string" {
            return tokens.get(start_index + 1).and_then(|inner| {
                let inner_tokens = tokenise(inner);
                (!inner_tokens.is_empty()).then_some(inner_tokens)
            });
        }
        if let Some(inner) = token.strip_prefix("--split-string=") {
            let inner_tokens = tokenise(inner);
            return (!inner_tokens.is_empty()).then_some(inner_tokens);
        }
        if token.contains('=') && !token.starts_with('-') {
            start_index += 1;
        } else if token.starts_with("--") {
            if token.contains('=') {
                // --chdir=/tmp: value is inline
                start_index += 1;
            } else if ENV_OPTIONS_WITH_VALUE.contains(&token.as_str())
                && start_index + 1 < tokens.len()
            {
                start_index += 2;
            } else {
                start_index += 1;
            }
        } else if token.starts_with('-') {
            if ENV_OPTIONS_WITH_VALUE.contains(&token.as_str()) && start_index + 1 < tokens.len() {
                start_index += 2;
            } else {
                start_index += 1;
            }
        } else {
            break;
        }
    }

    (start_index < tokens.len()).then(|| tokens[start_index..].to_vec())
}

#[must_use]
fn extract_interpreter_commands(tokens: &[String], interpreter: Option<&str>) -> Vec<String> {
    let Some(script) = tokens
        .iter()
        .position(|token| token == "-c" || token == "-e")
        .and_then(|index| tokens.get(index + 1))
    else {
        return Vec::new();
    };

    let pattern_strs = [
        r#"os\.system\s*\(\s*['\"](.*?)['\"]\s*\)"#,
        r#"subprocess\.(?:run|call|Popen)\s*\(\s*['\"](.*?)['\"]"#,
        r#"exec\s*\(\s*['\"](.*?)['\"]\s*\)"#,
        r#"execSync\s*\(\s*['\"](.*?)['\"]\s*\)"#,
        r"`([^`]+)`",
        r#"system\s*\(\s*['\"](.*?)['\"]\s*\)"#,
        r#"\beval\b\s*\(\s*['\"](.*?)['\"]\s*\)"#,
    ];

    let mut patterns: Vec<Regex> = pattern_strs
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    if interpreter
        .is_none_or(|cmd| SHELL_LIKE_INTERPRETERS.contains(&normalise_command_name(cmd).as_str()))
        && let Ok(re) = Regex::new(r"\$\(\s*(.*?)\s*\)")
    {
        patterns.push(re);
    }

    let mut results = Vec::new();
    for pattern in &patterns {
        for captures in pattern.captures_iter(script) {
            if let Some(inner) = captures.get(1) {
                let cmd = inner.as_str().to_string();
                if !results.contains(&cmd) {
                    results.push(cmd);
                }
            }
        }
    }

    results
}

const SUDO_SHORT_FLAGS_WITH_ARGS: &[&str] = &[
    "-u", "-g", "-C", "-h", "-p", "-r", "-t", "-T", "-U", "-D", "-a",
];
const SUDO_LONG_FLAGS_WITH_ARGS: &[&str] = &[
    "--user",
    "--group",
    "--close-from",
    "--host",
    "--prompt",
    "--role",
    "--type",
    "--command-timeout",
    "--other-user",
    "--chdir",
    "--login-class",
];

#[must_use]
fn extract_privileged_command(tokens: &[String]) -> Option<Vec<String>> {
    let mut start_index = 1;
    while start_index < tokens.len() {
        let token = &tokens[start_index];
        if token.starts_with("--") {
            if token.contains('=') {
                start_index += 1;
            } else if SUDO_LONG_FLAGS_WITH_ARGS.contains(&token.as_str())
                && start_index + 1 < tokens.len()
            {
                start_index += 2;
            } else {
                start_index += 1;
            }
        } else if token.starts_with('-') {
            if SUDO_SHORT_FLAGS_WITH_ARGS.contains(&token.as_str())
                && start_index + 1 < tokens.len()
            {
                start_index += 2;
            } else {
                start_index += 1;
            }
        } else {
            break;
        }
    }

    (start_index < tokens.len()).then(|| tokens[start_index..].to_vec())
}

#[must_use]
fn remaining_starts_with_recognised_wrapper(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return false;
    }
    let all_tokens = tokenise(trimmed);
    let prefix = skip_command_prefixes(&all_tokens, 0);
    let tokens = all_tokens[prefix..].to_vec();
    // Skip shell-style assignments so residual forms like `FOO=1 env rm ...`
    // still count as incomplete wrapper analysis at the depth limit.
    let Some(first) = tokens
        .iter()
        .find(|token| !is_environment_assignment(token))
    else {
        return false;
    };
    is_shell_wrapper(first)
        || is_privileged_wrapper(first)
        || is_env_wrapper(first)
        || is_interpreter(first)
}

#[must_use]
fn unwrap_command(cmd: &str, depth: usize) -> UnwrapResult {
    if depth >= MAX_UNWRAP_DEPTH {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
            // Fail closed when the residual still looks like a wrapper chain we
            // stopped peeling early; residual real commands are complete.
            incomplete: remaining_starts_with_recognised_wrapper(cmd),
        };
    }

    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
            incomplete: false,
        };
    }

    let tokens = tokenise(trimmed);
    let Some(first_token) = tokens.first() else {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
            incomplete: false,
        };
    };

    if is_shell_wrapper(first_token)
        && let Some(inner_cmd) = extract_shell_wrapper_arg(&tokens)
    {
        let inner = unwrap_command(&inner_cmd, depth + 1);
        let mut wrappers = vec![normalise_command_name(first_token)];
        wrappers.extend(inner.wrappers);
        return UnwrapResult {
            unwrapped: inner.unwrapped,
            wrappers,
            incomplete: inner.incomplete,
        };
    }

    if is_privileged_wrapper(first_token)
        && let Some(remaining) = extract_privileged_command(&tokens)
    {
        let inner = unwrap_command(&join_shell_tokens(&remaining), depth + 1);
        let mut wrappers = vec![normalise_command_name(first_token)];
        wrappers.extend(inner.wrappers);
        return UnwrapResult {
            unwrapped: inner.unwrapped,
            wrappers,
            incomplete: inner.incomplete,
        };
    }

    if is_env_wrapper(first_token)
        && let Some(remaining) = extract_env_command(&tokens)
    {
        let inner = unwrap_command(&join_shell_tokens(&remaining), depth + 1);
        let mut wrappers = vec![normalise_command_name(first_token)];
        wrappers.extend(inner.wrappers);
        return UnwrapResult {
            unwrapped: inner.unwrapped,
            wrappers,
            incomplete: inner.incomplete,
        };
    }

    if is_interpreter(first_token) {
        let inner_cmds = extract_interpreter_commands(&tokens, Some(first_token));
        if !inner_cmds.is_empty() {
            let joined = inner_cmds.join(" && ");
            let inner = unwrap_command(&joined, depth + 1);
            let mut wrappers = vec![normalise_command_name(first_token)];
            wrappers.extend(inner.wrappers);
            return UnwrapResult {
                unwrapped: inner.unwrapped,
                wrappers,
                incomplete: inner.incomplete,
            };
        }
    }

    UnwrapResult {
        unwrapped: trimmed.to_string(),
        wrappers: Vec::new(),
        incomplete: false,
    }
}

#[must_use]
fn expand_combined_flags(flags: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();

    for flag in flags {
        if flag.starts_with("--") {
            expanded.push(flag.clone());
        } else if flag.starts_with('-') && flag.len() > 2 {
            for chr in flag.chars().skip(1) {
                expanded.push(format!("-{chr}"));
            }
        } else {
            expanded.push(flag.clone());
        }
    }

    expanded
}

#[must_use]
fn split_at_separator(rest: &[String]) -> (Vec<String>, Vec<String>) {
    let separator_index = rest.iter().position(|token| token == "--");
    let (before_separator, after_separator) = match separator_index {
        Some(index) => (&rest[..index], &rest[index + 1..]),
        None => (rest, &[][..]),
    };

    let mut raw_flags = before_separator
        .iter()
        .filter(|token| token.starts_with('-'))
        .cloned()
        .collect::<Vec<_>>();
    if separator_index.is_some() {
        raw_flags.push("--".to_string());
    }
    let flags = expand_combined_flags(&raw_flags);

    let mut args = before_separator
        .iter()
        .filter(|token| !token.starts_with('-'))
        .cloned()
        .collect::<Vec<_>>();
    args.extend(after_separator.iter().cloned());

    (flags, args)
}

#[must_use]
fn is_likely_subcommand(arg: &str) -> bool {
    if arg.starts_with('-') {
        return false;
    }
    if arg.starts_with('/') || arg.starts_with("./") || arg.starts_with("../") {
        return false;
    }
    !arg.contains('=')
}

/// Global options that consume the next positional token as a value,
/// preventing it from being treated as a subcommand.
const GIT_GLOBAL_OPTIONS_WITH_VALUE: &[&str] = &[
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--exec-path",
];
const DOCKER_GLOBAL_OPTIONS_WITH_VALUE: &[&str] = &["-H", "--host", "--config", "--context"];

#[must_use]
fn global_options_for(command: &str) -> &'static [&'static str] {
    match command {
        "git" => GIT_GLOBAL_OPTIONS_WITH_VALUE,
        "docker" => DOCKER_GLOBAL_OPTIONS_WITH_VALUE,
        _ => &[],
    }
}

#[must_use]
fn extract_subcommand(command: &str, rest: &[String]) -> Option<String> {
    const COMMANDS_WITH_SUBCOMMANDS: &[&str] = &[
        "git", "npm", "yarn", "pnpm", "docker", "kubectl", "cargo", "go",
    ];

    if !COMMANDS_WITH_SUBCOMMANDS.contains(&command) {
        return None;
    }

    let global_opts = global_options_for(command);
    let mut skip_next = false;

    for token in rest {
        if skip_next {
            skip_next = false;
            continue;
        }
        if global_opts.contains(&token.as_str()) {
            skip_next = true;
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        if is_likely_subcommand(token) {
            return Some(token.clone());
        }
    }

    None
}

#[must_use]
fn parse_from_tokens(
    tokens: &[String],
    raw_cmd: &str,
    wrappers: &[String],
    unwrap_incomplete: bool,
) -> ParsedCommand {
    if tokens.is_empty() {
        return ParsedCommand {
            raw: raw_cmd.to_string(),
            command: String::new(),
            subcommand: None,
            flags: Vec::new(),
            args: Vec::new(),
            unwrapped: raw_cmd.to_string(),
            wrapper_chain: wrappers.to_vec(),
            unwrap_incomplete,
        };
    }

    let command_index = skip_command_prefixes(tokens, 0);

    if command_index >= tokens.len() {
        return ParsedCommand {
            raw: raw_cmd.to_string(),
            command: String::new(),
            subcommand: None,
            flags: Vec::new(),
            args: Vec::new(),
            unwrapped: tokens.join(" "),
            wrapper_chain: wrappers.to_vec(),
            unwrap_incomplete,
        };
    }

    let command = normalise_command_name(&tokens[command_index]);
    let rest = tokens[command_index + 1..].to_vec();
    let (flags, _args) = split_at_separator(&rest);
    let subcommand = extract_subcommand(&command, &rest);

    let global_opts = global_options_for(&command);
    let remaining_args = if command == "eval" {
        rest.iter()
            .filter(|token| token.as_str() != "--")
            .cloned()
            .collect()
    } else {
        let mut filtered = Vec::new();
        let mut skip_next = false;
        let mut past_separator = false;
        for token in &rest {
            if skip_next {
                skip_next = false;
                continue;
            }
            if !past_separator && token == "--" {
                past_separator = true;
                continue;
            }
            if past_separator {
                filtered.push(token.clone());
                continue;
            }
            if global_opts.contains(&token.as_str()) {
                skip_next = true;
                continue;
            }
            if token.starts_with('-') {
                continue;
            }
            if let Some(sub) = &subcommand
                && token == sub
            {
                continue;
            }
            filtered.push(token.clone());
        }
        filtered
    };

    ParsedCommand {
        raw: raw_cmd.to_string(),
        command,
        subcommand,
        flags,
        args: remaining_args,
        unwrapped: tokens.join(" "),
        wrapper_chain: wrappers.to_vec(),
        unwrap_incomplete,
    }
}

#[must_use]
pub fn parse_command(cmd: &str) -> ParsedCommand {
    let unwrap = unwrap_command(cmd, 0);
    let tokens = tokenise(&unwrap.unwrapped);

    if tokens.is_empty() {
        return ParsedCommand {
            raw: cmd.to_string(),
            command: String::new(),
            subcommand: None,
            flags: Vec::new(),
            args: Vec::new(),
            unwrapped: unwrap.unwrapped,
            wrapper_chain: unwrap.wrappers,
            unwrap_incomplete: unwrap.incomplete,
        };
    }

    parse_from_tokens(&tokens, cmd, &unwrap.wrappers, unwrap.incomplete)
}

#[must_use]
fn join_shell_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| shell_quote(token))
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
fn shell_quote(token: &str) -> String {
    if token.is_empty() {
        return "''".to_string();
    }
    if token.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t'
                | '&'
                | '|'
                | ';'
                | '('
                | ')'
                | '<'
                | '>'
                | '\''
                | '"'
                | '\\'
                | '`'
                | '$'
                | '!'
                | '{'
                | '}'
                | '*'
                | '?'
                | '['
                | '#'
                | '~'
        )
    }) {
        let escaped = token.replace('\'', "'\\''");
        return format!("'{escaped}'");
    }
    token.to_string()
}

fn structural_shell_body(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if let Some(open) = trimmed.find('{') {
        let prefix = trimmed[..open].trim_end();
        if is_function_declaration_prefix(prefix) {
            let close = find_function_closing_brace(trimmed, open)?;
            if close > open {
                let body = trimmed[open + 1..close].trim();
                let suffix_raw = &trimmed[close + 1..];
                let suffix = suffix_raw.trim();
                return Some(if suffix.is_empty() {
                    body.to_string()
                } else if suffix.starts_with([';', '|', '&']) {
                    format!("{body} {suffix}")
                } else {
                    format!("{body}; {suffix}")
                });
            }
        }
    }
    case_arm_bodies(trimmed)
}

fn find_function_closing_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    for (offset, character) in text[open + 1..].char_indices() {
        let index = open + 1 + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '{' if !in_single_quote && !in_double_quote => depth += 1,
            '}' if !in_single_quote && !in_double_quote => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn case_arm_bodies(command: &str) -> Option<String> {
    let rest = command.strip_prefix("case")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (_, arms_start) = find_unquoted_reserved_word(rest, "in")?;
    let arms = rest[arms_start..].trim_start();
    let arms = arms.strip_suffix("esac")?.trim_end();
    let mut bodies = Vec::new();
    let mut cursor = 0usize;
    while cursor < arms.len() {
        let close = find_case_pattern_close(arms, cursor)?;
        let body_start = close + 1;
        let (body_end, next) = find_case_arm_end(arms, body_start);
        let body = arms[body_start..body_end].trim();
        if !body.is_empty() {
            bodies.push(body);
        }
        if next <= cursor {
            break;
        }
        cursor = next;
    }
    (!bodies.is_empty()).then(|| bodies.join("; "))
}

fn find_unquoted_reserved_word(text: &str, needle: &str) -> Option<(usize, usize)> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut word_start = None;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            word_start = None;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            word_start = None;
            continue;
        }
        match character {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                word_start = None;
                continue;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                word_start = None;
                continue;
            }
            _ => {}
        }
        if in_single_quote || in_double_quote {
            continue;
        }
        if character == '_' || character.is_ascii_alphabetic() {
            word_start.get_or_insert(index);
            continue;
        }
        if character.is_ascii_alphanumeric() {
            continue;
        }
        if let Some(start) = word_start.take()
            && &text[start..index] == needle
            && (character.is_whitespace() || character == ';')
        {
            let end = index + usize::from(character == ';');
            return Some((start, end));
        }
    }
    None
}

fn find_case_pattern_close(text: &str, start: usize) -> Option<usize> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (offset, character) in text[start..].char_indices() {
        let index = start + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '(' if !in_single_quote && !in_double_quote => depth += 1,
            ')' if !in_single_quote && !in_double_quote && depth == 0 => return Some(index),
            ')' if !in_single_quote && !in_double_quote => depth -= 1,
            _ => {}
        }
    }
    None
}

fn find_case_arm_end(text: &str, start: usize) -> (usize, usize) {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    for (offset, character) in text[start..].char_indices() {
        let index = start + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '(' if !in_single_quote && !in_double_quote => paren_depth += 1,
            ')' if !in_single_quote && !in_double_quote && paren_depth > 0 => paren_depth -= 1,
            '{' if !in_single_quote && !in_double_quote => brace_depth += 1,
            '}' if !in_single_quote && !in_double_quote && brace_depth > 0 => brace_depth -= 1,
            ';' if !in_single_quote && !in_double_quote && paren_depth == 0 && brace_depth == 0 => {
                let suffix = &text[index..];
                let terminator_len = if suffix.starts_with(";;&") {
                    3
                } else if suffix.starts_with(";;") || suffix.starts_with(";&") {
                    2
                } else {
                    0
                };
                if terminator_len > 0 && case_reserved_word_balance(&text[start..index]) <= 0 {
                    return (index, index + terminator_len);
                }
            }
            _ => {}
        }
    }
    (text.len(), text.len())
}

#[must_use]
pub fn parse_compound_command(cmd: &str) -> CompoundCommandResult {
    if let Some(body) = structural_shell_body(cmd) {
        return parse_compound_command(&body);
    }
    let tokenised = tokenise_with_operators(cmd);

    if !tokenised.is_compound || tokenised.sub_commands.len() <= 1 {
        // Not compound at the top level — but unwrapping (e.g. bash -c) may
        // reveal inner operators.  Re-check the unwrapped command.
        let unwrap = unwrap_command(cmd, 0);
        if !unwrap.wrappers.is_empty() {
            let inner = tokenise_with_operators(&unwrap.unwrapped);
            if inner.is_compound && inner.sub_commands.len() > 1 {
                let mut commands = Vec::new();
                let mut operators = Vec::new();
                for sub in inner.sub_commands {
                    if let Some(op) = sub.operator {
                        operators.push(op);
                    }
                    if !sub.tokens.is_empty() {
                        let raw_sub = sub.tokens.join(" ");
                        let tokens = tokenise(&raw_sub);
                        let parsed = parse_from_tokens(
                            &tokens,
                            &raw_sub,
                            &unwrap.wrappers,
                            unwrap.incomplete,
                        );
                        commands.push(parsed);
                    }
                }
                return CompoundCommandResult {
                    is_compound: true,
                    commands,
                    operators,
                    raw: cmd.to_string(),
                };
            }
        }
        return CompoundCommandResult {
            is_compound: false,
            commands: vec![parse_command(cmd)],
            operators: Vec::new(),
            raw: cmd.to_string(),
        };
    }

    let mut commands = Vec::new();
    let mut operators = Vec::new();

    for sub_command in tokenised.sub_commands {
        if let Some(operator) = sub_command.operator {
            operators.push(operator);
        }
        if !sub_command.tokens.is_empty() {
            let raw_sub = sub_command
                .tokens
                .iter()
                .map(|t| shell_quote(t))
                .collect::<Vec<_>>()
                .join(" ");
            let unwrap = unwrap_command(&raw_sub, 0);
            // Re-check unwrapped result for inner operators
            let inner = tokenise_with_operators(&unwrap.unwrapped);
            if inner.is_compound && inner.sub_commands.len() > 1 {
                for sub in inner.sub_commands {
                    if let Some(op) = sub.operator {
                        operators.push(op);
                    }
                    if !sub.tokens.is_empty() {
                        let inner_raw = sub
                            .tokens
                            .iter()
                            .map(|t| shell_quote(t))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let tokens = tokenise(&inner_raw);
                        let parsed = parse_from_tokens(
                            &tokens,
                            &inner_raw,
                            &unwrap.wrappers,
                            unwrap.incomplete,
                        );
                        commands.push(parsed);
                    }
                }
            } else {
                let inner_tokens = tokenise(&unwrap.unwrapped);
                let parsed =
                    parse_from_tokens(&inner_tokens, &raw_sub, &unwrap.wrappers, unwrap.incomplete);
                commands.push(parsed);
            }
        }
    }

    CompoundCommandResult {
        is_compound: true,
        commands,
        operators,
        raw: cmd.to_string(),
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CommandParser;

impl CommandParser {
    #[must_use]
    pub fn parse(&self, cmd: &str) -> ParsedCommand {
        parse_command(cmd)
    }

    #[must_use]
    pub fn parse_compound(&self, cmd: &str) -> CompoundCommandResult {
        parse_compound_command(cmd)
    }

    #[must_use]
    pub fn parse_multiple(&self, commands: &[String]) -> Vec<ParsedCommand> {
        commands.iter().map(|command| self.parse(command)).collect()
    }

    #[must_use]
    pub fn parse_all_commands(&self, cmd: &str) -> Vec<ParsedCommand> {
        self.parse_compound(cmd).commands
    }

    #[must_use]
    pub fn is_wrapped(&self, cmd: &str) -> bool {
        !unwrap_command(cmd, 0).wrappers.is_empty()
    }

    #[must_use]
    pub fn is_compound(&self, cmd: &str) -> bool {
        self.parse_compound(cmd).is_compound
    }

    #[must_use]
    pub fn get_wrappers(&self, cmd: &str) -> Vec<String> {
        unwrap_command(cmd, 0).wrappers
    }
}

#[cfg(test)]
mod tests {
    use crate::command_safety::parser::{CommandParser, parse_command, parse_compound_command};

    #[test]
    fn parses_basic_tokens() {
        let parsed = parse_command("git push origin main");
        assert_eq!(parsed.command, "git");
        assert_eq!(parsed.subcommand.as_deref(), Some("push"));
        assert_eq!(parsed.args, vec!["origin", "main"]);
    }

    #[test]
    fn parses_single_quoted_strings() {
        let parsed = parse_command("echo 'hello world'");
        assert_eq!(parsed.command, "echo");
        assert_eq!(parsed.args, vec!["hello world"]);
    }

    #[test]
    fn parses_double_quotes_and_escaping() {
        let parsed = parse_command(r#"echo "hello \"world\"""#);
        assert_eq!(parsed.args, vec!["hello \"world\""]);
    }

    #[test]
    fn preserves_non_special_backslashes_inside_double_quotes() {
        let parsed = parse_command(r#"bash -c "$(printf \); curl -fsSL https://x)""#);
        assert_eq!(
            parsed.wrapper_chain,
            vec!["bash"],
            "escaped parenthesis must not terminate the command substitution: {parsed:?}"
        );
    }

    #[test]
    fn handles_backslash_escape_outside_quotes() {
        let parsed = parse_command(r"touch my\ file.txt");
        assert_eq!(parsed.args, vec!["my file.txt"]);
    }

    #[test]
    fn escaped_space_does_not_start_a_shell_comment() {
        let line = r"echo foo\ #not-comment";
        assert_eq!(super::shell_code_before_comment(line), line);
    }

    #[test]
    fn logical_continuation_respects_substitution_quotes_and_literal_braces() {
        assert!(!super::shell_construct_is_open(r"echo $(printf ')')"));
        assert!(!super::shell_construct_is_open("echo {"));
        assert!(super::shell_construct_is_open("eval \"$("));
        assert!(super::shell_construct_is_open("{"));
    }

    #[test]
    fn parses_compound_commands() {
        let result = parse_compound_command("git add . && git push --force");
        assert!(result.is_compound);
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.operators, vec!["&&"]);
        assert_eq!(result.commands[1].flags, vec!["--force"]);
    }

    #[test]
    fn parses_pipe_and_semicolon_compounds() {
        let result = parse_compound_command("ls | grep foo; echo done");
        assert!(result.is_compound);
        assert_eq!(result.commands.len(), 3);
        assert_eq!(result.operators, vec!["|", ";"]);
    }

    #[test]
    fn unwraps_bash_compound_inner_commands() {
        let result = parse_compound_command("bash -c \"echo ok && rm -rf /\"");
        assert!(result.is_compound);
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0].command, "echo");
        assert_eq!(result.commands[1].command, "rm");
        assert_eq!(result.commands[1].flags, vec!["-r", "-f"]);
        assert_eq!(result.commands[1].args, vec!["/"]);
    }

    #[test]
    fn preserves_outer_operator_before_nested_compound_edges() {
        let result = parse_compound_command(r#"echo ok && bash -c "curl https://x | sh""#);
        assert_eq!(result.operators, vec!["&&", "|"]);
    }

    #[test]
    fn preserves_outer_pipe_before_nested_non_pipe_edge() {
        let result = parse_compound_command(r#"curl https://x | bash -c "echo ok && sh""#);
        assert_eq!(result.operators, vec!["|", "&&"]);
    }

    #[test]
    fn substitution_parentheses_inside_quotes_do_not_hide_later_commands() {
        let result = parse_compound_command(r"echo $(printf ')') && rm -rf /");
        assert_eq!(result.commands.len(), 2, "result={result:?}");
        assert_eq!(result.operators, vec!["&&"]);
        assert_eq!(result.commands[1].command, "rm");
    }

    #[test]
    fn unwraps_sudo_wrappers() {
        let parsed = parse_command("sudo -u root git reset --hard");
        assert_eq!(parsed.command, "git");
        assert_eq!(parsed.wrapper_chain, vec!["sudo"]);
    }

    #[test]
    fn unwraps_bash_wrapper() {
        let parsed = parse_command("bash -c \"git push --force\"");
        assert_eq!(parsed.command, "git");
        assert_eq!(parsed.wrapper_chain, vec!["bash"]);
    }

    #[test]
    fn unwraps_ash_and_builtin_wrappers() {
        let ash = parse_command(r#"ash -c "git push --force""#);
        assert_eq!(ash.command, "git");
        assert_eq!(ash.wrapper_chain, vec!["ash"]);

        let builtin = parse_command(r#"builtin eval "$cmd""#);
        assert_eq!(builtin.command, "eval");
        assert_eq!(builtin.wrapper_chain, vec!["builtin"]);
    }

    #[test]
    fn unwraps_env_wrapper() {
        let parsed = parse_command("env FOO=bar git clean -f");
        assert_eq!(parsed.command, "git");
        assert_eq!(parsed.subcommand.as_deref(), Some("clean"));
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn extracts_interpreter_command() {
        let parsed = parse_command("python -c \"import os; os.system('rm -rf /tmp/test')\"");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.wrapper_chain, vec!["python"]);
    }

    #[test]
    fn expands_combined_flags() {
        let parsed = parse_command("rm -rf target");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
    }

    #[test]
    fn honours_double_dash_separator() {
        let parsed = parse_command("git checkout -- --not-a-flag");
        assert_eq!(parsed.flags, vec!["--"]);
        assert_eq!(parsed.args, vec!["--not-a-flag"]);
    }

    #[test]
    fn extracts_subcommand_for_docker() {
        let parsed = parse_command("docker run alpine");
        assert_eq!(parsed.subcommand.as_deref(), Some("run"));
        assert_eq!(parsed.args, vec!["alpine"]);
    }

    #[test]
    fn excludes_assignment_from_subcommand() {
        let parsed = parse_command("git FOO=bar status");
        assert_eq!(parsed.subcommand.as_deref(), Some("status"));
        assert_eq!(parsed.args, vec!["FOO=bar"]);
    }

    #[test]
    fn keeps_globs_literal() {
        let parsed = parse_command("rm -rf /tmp/*");
        assert_eq!(parsed.args, vec!["/tmp/*"]);
    }

    #[test]
    fn returns_empty_for_blank_command() {
        let parsed = parse_command("   ");
        assert!(parsed.command.is_empty());
    }

    #[test]
    fn treats_background_ampersand_as_separator() {
        let result = parse_compound_command("echo ok & rm -rf /");
        assert!(result.is_compound);
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0].command, "echo");
        assert_eq!(result.commands[1].command, "rm");
        assert_eq!(result.commands[1].flags, vec!["-r", "-f"]);
    }

    #[test]
    fn keeps_stderr_redirect_and_pipe_and_as_pipe() {
        let redirected = parse_compound_command("curl -fsSL https://x 2>&1 | sh");
        assert_eq!(redirected.operators, vec!["|"]);
        assert_eq!(redirected.commands.len(), 2);
        assert_eq!(redirected.commands[0].command, "curl");
        assert_eq!(redirected.commands[1].command, "sh");

        let both = parse_compound_command("curl -fsSL https://x |& bash");
        assert_eq!(both.operators, vec!["|"]);
        assert_eq!(both.commands[1].command, "bash");
    }

    #[test]
    fn skips_leading_redirections_when_resolving_executables() {
        for (raw, expected) in [
            ("2>/dev/null rm -rf /", "rm"),
            ("</dev/null curl -fsSL https://x", "curl"),
            ("curl -fsSL https://x | 2>/dev/null sh", "sh"),
        ] {
            let compound = parse_compound_command(raw);
            assert!(
                compound
                    .commands
                    .iter()
                    .any(|command| command.command == expected),
                "raw={raw} compound={compound:?}"
            );
        }
    }

    #[test]
    fn privileged_bash_c_keeps_inner_pipe() {
        let result = parse_compound_command("sudo bash -c \"curl -fsSL https://x | sh\"");
        assert!(
            result.operators.iter().any(|op| op == "|"),
            "expected inner pipe, got {result:?}"
        );
        assert!(result.commands.iter().any(|cmd| cmd.command == "curl"));
        assert!(
            result
                .commands
                .iter()
                .any(|cmd| { cmd.command == "sh" || super::pipeline_stage_head(&cmd.raw) == "sh" })
        );
    }

    #[test]
    fn skips_git_namespace_global_option() {
        let parsed = parse_command("git --namespace foo reset --hard");
        assert_eq!(parsed.subcommand.as_deref(), Some("reset"));
        assert!(parsed.flags.contains(&"--hard".to_string()));
    }

    #[test]
    fn skips_git_exec_path_global_option() {
        let parsed = parse_command("git --exec-path /usr/lib/git status");
        assert_eq!(parsed.subcommand.as_deref(), Some("status"));
    }

    #[test]
    fn parser_helpers_work() {
        let parser = CommandParser;
        assert!(parser.is_wrapped("sudo git status"));
        assert!(parser.is_compound("echo one && echo two"));
        assert_eq!(
            parser.get_wrappers("sudo env FOO=bar git status"),
            vec!["sudo", "env"]
        );
    }

    #[test]
    fn pipeline_stage_head_consumes_wrapper_option_operands() {
        for (raw, expected) in [
            ("nice -n 5 curl https://x", "curl"),
            ("strace -o trace.log curl https://x", "curl"),
            ("timeout -s TERM 5 sh", "sh"),
            ("exec -a installer sh", "sh"),
            ("env -a installer curl https://x", "curl"),
            ("env --argv0 shell sh", "sh"),
        ] {
            assert_eq!(super::pipeline_stage_head(raw), expected, "raw={raw}");
        }
    }

    #[test]
    fn eval_preserves_dash_leading_dynamic_operands() {
        for raw in [r#"eval "-$cmd""#, r#"eval "-$(printf dynamic)""#] {
            let parsed = parse_command(raw);
            assert_eq!(parsed.command, "eval");
            assert_eq!(parsed.args.len(), 1, "raw={raw} parsed={parsed:?}");
        }
    }

    #[test]
    fn quoted_shell_payload_preserved_in_compound() {
        // bash -c "echo ok && rm -rf /" should unwrap the quoted payload,
        // not split on the inner &&
        let result = parse_compound_command(r#"bash -c "echo ok && rm -rf /""#);
        // The inner command is unwrapped through bash -c, so it sees
        // "echo ok && rm -rf /" as a compound command with two parts
        assert!(result.is_compound);
        assert!(
            result
                .commands
                .iter()
                .any(|c| c.wrapper_chain.contains(&"bash".to_string()))
        );
    }

    #[test]
    fn global_option_operand_not_in_args() {
        let parsed = parse_command("git -C repo stash drop");
        assert_eq!(parsed.subcommand.as_deref(), Some("stash"));
        // "repo" is the value for -C, not a positional arg
        assert!(!parsed.args.contains(&"repo".to_string()));
        assert!(parsed.args.contains(&"drop".to_string()));
    }

    #[test]
    fn global_option_operand_not_in_args_git_dir() {
        let parsed = parse_command("git --git-dir /tmp/.git log --oneline");
        assert_eq!(parsed.subcommand.as_deref(), Some("log"));
        assert!(!parsed.args.contains(&"/tmp/.git".to_string()));
    }

    #[test]
    fn unwraps_sudo_long_option_with_separate_value() {
        let parsed = parse_command("sudo --user root rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
        assert_eq!(parsed.wrapper_chain, vec!["sudo"]);
    }

    #[test]
    fn unwraps_sudo_long_option_with_equals_value() {
        let parsed = parse_command("sudo --user=root rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.wrapper_chain, vec!["sudo"]);
    }

    #[test]
    fn unwraps_env_chdir_option_with_separate_value() {
        let parsed = parse_command("env -C /tmp rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn unwraps_env_long_chdir_option() {
        let parsed = parse_command("env --chdir /tmp rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn unwraps_env_split_string_as_inner_command() {
        let parsed = parse_command(r#"env -S "rm -rf /""#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn unwraps_env_long_split_string_option() {
        let parsed = parse_command(r#"env --split-string "rm -rf /""#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn unwraps_env_split_string_equals_form() {
        let parsed = parse_command(r#"env --split-string="rm -rf /""#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.wrapper_chain, vec!["env"]);
    }

    #[test]
    fn strips_leading_environment_assignments() {
        let parsed = parse_command("FOO=bar BAR=baz rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
    }

    #[test]
    fn depth_limited_nested_env_marks_unwrap_incomplete() {
        // Six nested env wrappers exceed MAX_UNWRAP_DEPTH (5) and must not
        // report a residual wrapper as a complete parse.
        let parsed = parse_command("env env env env env env rm -rf /");
        assert!(
            parsed.unwrap_incomplete || parsed.command == "rm",
            "expected incomplete unwrap or full peel to rm; got command={:?} incomplete={}",
            parsed.command,
            parsed.unwrap_incomplete
        );
        if parsed.unwrap_incomplete {
            // Residual must not be treated as a trusted complete analysis.
            assert_ne!(
                parsed.command, "rm",
                "incomplete flag with command=rm would be inconsistent"
            );
        }
    }

    #[test]
    fn five_nested_env_wrappers_still_fully_unwrap() {
        // Depth budget allows five peels; this remains a complete analysis.
        let parsed = parse_command("env env env env env rm -rf /");
        assert!(!parsed.unwrap_incomplete);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
    }

    #[test]
    fn depth_limit_with_assignment_prefixed_residual_is_incomplete() {
        // After five peels the residual is `FOO=1 env rm -rf /`. The incomplete
        // detector must look past the assignment to the residual wrapper.
        let parsed = parse_command("env env env env env FOO=1 env rm -rf /");
        assert!(
            parsed.unwrap_incomplete,
            "assignment-prefixed residual wrapper must be incomplete; command={:?}",
            parsed.command
        );
    }

    #[test]
    fn normalises_absolute_executable_path_to_basename() {
        let parsed = parse_command("/bin/rm -rf /");
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
    }

    #[test]
    fn normalises_usr_bin_path_and_relative_path_forms() {
        assert_eq!(parse_command("/usr/bin/rm -rf /").command, "rm");
        assert_eq!(parse_command("./rm -rf /").command, "rm");
        assert_eq!(parse_command("../sbin/rm -rf /").command, "rm");
    }

    #[test]
    fn unwraps_shell_wrapper_with_absolute_path() {
        let parsed = parse_command(r#"/bin/bash -c "rm -rf /""#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["/"]);
        assert_eq!(parsed.wrapper_chain, vec!["bash"]);
    }

    #[test]
    fn preserves_command_substitution_in_quoted_argument() {
        // bash -c payload keeps the double-quoted substitution as a single arg.
        let parsed = parse_command(r#"bash -c 'rm -rf "$(printf /)"'"#);
        assert_eq!(parsed.command, "rm");
        assert_eq!(parsed.flags, vec!["-r", "-f"]);
        assert_eq!(parsed.args, vec!["$(printf /)"]);
    }
}
