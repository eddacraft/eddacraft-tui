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
    SHELL_WRAPPERS.contains(&cmd)
}

#[must_use]
fn is_privileged_wrapper(cmd: &str) -> bool {
    PRIVILEGED_WRAPPERS.contains(&cmd)
}

#[must_use]
fn is_env_wrapper(cmd: &str) -> bool {
    ENV_WRAPPERS.contains(&cmd)
}

#[must_use]
fn is_interpreter(cmd: &str) -> bool {
    INTERPRETER_COMMANDS.contains(&cmd)
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

#[must_use]
fn tokenise_shell(cmd: &str) -> Vec<ShellToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = cmd.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(ch) = chars.next() {
        if in_single_quote {
            if ch == '\'' {
                in_single_quote = false;
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
            '\'' => in_single_quote = true,
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
                if chars.peek() == Some(&'&') {
                    let _ = chars.next();
                    if !current.is_empty() {
                        tokens.push(ShellToken::Word(std::mem::take(&mut current)));
                    }
                    tokens.push(ShellToken::Operator("&&".to_string()));
                } else {
                    current.push('&');
                }
            }
            '|' => {
                if !current.is_empty() {
                    tokens.push(ShellToken::Word(std::mem::take(&mut current)));
                }
                if chars.peek() == Some(&'|') {
                    let _ = chars.next();
                    tokens.push(ShellToken::Operator("||".to_string()));
                } else {
                    tokens.push(ShellToken::Operator("|".to_string()));
                }
            }
            ';' => {
                if !current.is_empty() {
                    tokens.push(ShellToken::Word(std::mem::take(&mut current)));
                }
                tokens.push(ShellToken::Operator(";".to_string()));
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(ShellToken::Word(std::mem::take(&mut current)));
                }
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
    let mut start_index = 1;
    while start_index < tokens.len() {
        let token = &tokens[start_index];
        if token.contains('=') || token.starts_with('-') {
            start_index += 1;
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

    if interpreter.is_none_or(|cmd| SHELL_LIKE_INTERPRETERS.contains(&cmd)) {
        if let Ok(re) = Regex::new(r"\$\(\s*(.*?)\s*\)") {
            patterns.push(re);
        }
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

#[must_use]
fn unwrap_command(cmd: &str, depth: usize) -> UnwrapResult {
    if depth >= MAX_UNWRAP_DEPTH {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
        };
    }

    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
        };
    }

    let tokens = tokenise(trimmed);
    let Some(first_token) = tokens.first() else {
        return UnwrapResult {
            unwrapped: cmd.to_string(),
            wrappers: Vec::new(),
        };
    };

    if is_shell_wrapper(first_token)
        && let Some(inner_cmd) = extract_shell_wrapper_arg(&tokens)
    {
        let inner = unwrap_command(&inner_cmd, depth + 1);
        let mut wrappers = vec![first_token.clone()];
        wrappers.extend(inner.wrappers);
        return UnwrapResult {
            unwrapped: inner.unwrapped,
            wrappers,
        };
    }

    if is_privileged_wrapper(first_token) {
        let sudo_flags_with_args = [
            "-u", "-g", "-C", "-h", "-p", "-r", "-t", "-T", "-U", "-D", "-a",
        ];
        let mut start_index = 1;
        while start_index < tokens.len() {
            let token = &tokens[start_index];
            if token.starts_with('-') {
                if sudo_flags_with_args.contains(&token.as_str()) && start_index + 1 < tokens.len()
                {
                    start_index += 2;
                } else {
                    start_index += 1;
                }
            } else {
                break;
            }
        }

        if start_index < tokens.len() {
            let remaining = tokens[start_index..].join(" ");
            let inner = unwrap_command(&remaining, depth + 1);
            let mut wrappers = vec![first_token.clone()];
            wrappers.extend(inner.wrappers);
            return UnwrapResult {
                unwrapped: inner.unwrapped,
                wrappers,
            };
        }
    }

    if is_env_wrapper(first_token)
        && let Some(remaining) = extract_env_command(&tokens)
    {
        let inner = unwrap_command(&remaining.join(" "), depth + 1);
        let mut wrappers = vec![first_token.clone()];
        wrappers.extend(inner.wrappers);
        return UnwrapResult {
            unwrapped: inner.unwrapped,
            wrappers,
        };
    }

    if is_interpreter(first_token) {
        let inner_cmds = extract_interpreter_commands(&tokens, Some(first_token));
        if !inner_cmds.is_empty() {
            let joined = inner_cmds.join(" && ");
            let inner = unwrap_command(&joined, depth + 1);
            let mut wrappers = vec![first_token.clone()];
            wrappers.extend(inner.wrappers);
            return UnwrapResult {
                unwrapped: inner.unwrapped,
                wrappers,
            };
        }
    }

    UnwrapResult {
        unwrapped: trimmed.to_string(),
        wrappers: Vec::new(),
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
const GIT_GLOBAL_OPTIONS_WITH_VALUE: &[&str] = &["-C", "-c", "--git-dir", "--work-tree"];
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
fn parse_from_tokens(tokens: &[String], raw_cmd: &str, wrappers: &[String]) -> ParsedCommand {
    if tokens.is_empty() {
        return ParsedCommand {
            raw: raw_cmd.to_string(),
            command: String::new(),
            subcommand: None,
            flags: Vec::new(),
            args: Vec::new(),
            unwrapped: raw_cmd.to_string(),
            wrapper_chain: wrappers.to_vec(),
        };
    }

    let command = tokens[0].clone();
    let rest = tokens[1..].to_vec();
    let (flags, args) = split_at_separator(&rest);
    let subcommand = extract_subcommand(&command, &rest);

    let remaining_args = if let Some(sub) = &subcommand {
        args.into_iter()
            .filter(|arg| arg != sub)
            .collect::<Vec<_>>()
    } else {
        args
    };

    ParsedCommand {
        raw: raw_cmd.to_string(),
        command,
        subcommand,
        flags,
        args: remaining_args,
        unwrapped: tokens.join(" "),
        wrapper_chain: wrappers.to_vec(),
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
        };
    }

    parse_from_tokens(&tokens, cmd, &unwrap.wrappers)
}

#[must_use]
pub fn parse_compound_command(cmd: &str) -> CompoundCommandResult {
    let tokenised = tokenise_with_operators(cmd);

    if !tokenised.is_compound || tokenised.sub_commands.len() <= 1 {
        return CompoundCommandResult {
            is_compound: false,
            commands: vec![parse_command(cmd)],
            operators: Vec::new(),
        };
    }

    let mut commands = Vec::new();
    let mut operators = Vec::new();

    for sub_command in tokenised.sub_commands {
        if !sub_command.tokens.is_empty() {
            let raw_sub = sub_command.tokens.join(" ");
            let unwrap = unwrap_command(&raw_sub, 0);
            let inner_tokens = tokenise(&unwrap.unwrapped);
            let parsed = parse_from_tokens(&inner_tokens, &raw_sub, &unwrap.wrappers);
            commands.push(parsed);
        }
        if let Some(operator) = sub_command.operator {
            operators.push(operator);
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
    fn parser_helpers_work() {
        let parser = CommandParser;
        assert!(parser.is_wrapped("sudo git status"));
        assert!(parser.is_compound("echo one && echo two"));
        assert_eq!(
            parser.get_wrappers("sudo env FOO=bar git status"),
            vec!["sudo", "env"]
        );
    }
}
