use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::command_safety::types::ParsedCommand;

const MAX_UNWRAP_DEPTH: usize = 5;

const SHELL_WRAPPERS: &[&str] = &["bash", "sh", "zsh", "dash"];
const PRIVILEGED_WRAPPERS: &[&str] = &["sudo", "doas"];
const ENV_WRAPPERS: &[&str] = &["env", "command", "nohup", "nice", "time", "strace"];
const INTERPRETER_COMMANDS: &[&str] = &["python", "python3", "node", "ruby", "perl", "php"];
const SHELL_LIKE_INTERPRETERS: &[&str] = &["bash", "sh", "zsh", "dash"];

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
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut previous: Option<char> = None;

    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            previous = Some(character);
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            previous = Some(character);
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '#' if !in_single_quote
                && !in_double_quote
                && previous.is_none_or(|before| {
                    before.is_whitespace() || matches!(before, ';' | '|' | '&' | '(' | ')')
                }) =>
            {
                return &line[..index];
            }
            _ => {}
        }
        previous = Some(character);
    }
    line
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

const PIPELINE_GRAMMAR_PREFIXES: &[&str] = &[
    "if", "then", "else", "elif", "fi", "while", "until", "do", "done", "!", "builtin",
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
        "env" => matches!(flag, "-C" | "--chdir" | "-u" | "--unset"),
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
pub fn pipeline_stage_head(raw: &str) -> String {
    let tokens = tokenise(raw);
    let mut index = 0;
    while index < tokens.len() {
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
            return normalise_command_name(&tokens[index + 1]);
        }
        return name;
    }
    String::new()
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

#[must_use]
fn tokenise_shell(cmd: &str) -> Vec<ShellToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = cmd.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut preserve_single_quote = false;

    while let Some(ch) = chars.next() {
        if in_single_quote {
            if ch == '\'' {
                in_single_quote = false;
                close_single_quote(&mut current, &mut preserve_single_quote);
            } else {
                current.push(ch);
            }
            continue;
        }

        if in_double_quote {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
                continue;
            }
            if ch == '"' {
                in_double_quote = false;
            } else {
                current.push(ch);
            }
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
            }
            '#' => {
                if current.is_empty() {
                    break;
                }
                current.push('#');
            }
            '&' => {
                // `2>&1` / `>&1` are redirections, not background separators.
                if current.ends_with('>') {
                    current.push('&');
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                    continue;
                }
                if chars.peek() == Some(&'>') {
                    current.push('&');
                    current.push(chars.next().expect("peeked >"));
                    continue;
                }
                push_current_word(&mut tokens, &mut current);
                if chars.peek() == Some(&'&') {
                    let _ = chars.next();
                    tokens.push(ShellToken::Operator("&&".to_string()));
                } else {
                    tokens.push(ShellToken::Operator("&".to_string()));
                }
            }
            '|' => {
                push_current_word(&mut tokens, &mut current);
                if chars.peek() == Some(&'|') {
                    let _ = chars.next();
                    tokens.push(ShellToken::Operator("||".to_string()));
                } else if chars.peek() == Some(&'&') {
                    // bash `|&` is a pipe of stdout+stderr, not a background `&`.
                    let _ = chars.next();
                    tokens.push(ShellToken::Operator("|".to_string()));
                } else {
                    tokens.push(ShellToken::Operator("|".to_string()));
                }
            }
            ';' => {
                push_current_word(&mut tokens, &mut current);
                tokens.push(ShellToken::Operator(";".to_string()));
            }
            c if c.is_whitespace() => {
                push_current_word(&mut tokens, &mut current);
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(ShellToken::Word(current));
    }

    tokens
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
fn extract_shell_wrapper_arg(tokens: &[String]) -> Option<String> {
    for (index, token) in tokens.iter().enumerate() {
        if token == "-c" {
            return tokens.get(index + 1).cloned();
        }
        if token.starts_with('-') && !token.starts_with("--") && token.ends_with('c') {
            return tokens.get(index + 1).cloned();
        }
    }
    None
}

#[must_use]
fn extract_env_command(tokens: &[String]) -> Option<Vec<String>> {
    const ENV_OPTIONS_WITH_VALUE: &[&str] = &["-C", "--chdir", "-u", "--unset"];

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
    let tokens = tokenise(trimmed);
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

    let command_index = tokens
        .iter()
        .take_while(|token| is_environment_assignment(token))
        .count();

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
    let remaining_args = {
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

#[must_use]
pub fn parse_compound_command(cmd: &str) -> CompoundCommandResult {
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
                };
            }
        }
        return CompoundCommandResult {
            is_compound: false,
            commands: vec![parse_command(cmd)],
            operators: Vec::new(),
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
    fn handles_backslash_escape_outside_quotes() {
        let parsed = parse_command(r"touch my\ file.txt");
        assert_eq!(parsed.args, vec!["my file.txt"]);
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
        ] {
            assert_eq!(super::pipeline_stage_head(raw), expected, "raw={raw}");
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
