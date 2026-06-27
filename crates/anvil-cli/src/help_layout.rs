use clap::Command;

/// Return every non-hidden command path exposed by clap, including nested
/// subcommands but excluding hidden compatibility aliases.
pub fn visible_command_paths(root: &Command) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    let mut prefix = vec![root.get_name().to_owned()];
    collect_visible_command_paths(root, &mut prefix, &mut paths);
    paths.sort();
    paths
}

pub fn contains_path(paths: &[Vec<String>], expected: &[&str]) -> bool {
    paths.iter().any(|path| {
        path.len() == expected.len()
            && path
                .iter()
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected)
    })
}

fn collect_visible_command_paths(
    command: &Command,
    prefix: &mut Vec<String>,
    paths: &mut Vec<Vec<String>>,
) {
    for subcommand in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        prefix.push(subcommand.get_name().to_owned());
        paths.push(prefix.clone());
        collect_visible_command_paths(subcommand, prefix, paths);
        prefix.pop();
    }
}
