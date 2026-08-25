use std::collections::BTreeSet;

use anvil_hook::HookKind;
use anvil_intercept::ipc::{CANONICAL_GOVERNED_METHODS, DeliveryVisibility};
use clap::{Arg, Command, CommandFactory};
use serde_json::Value;

use crate::Cli;
use crate::activation::agent_registry::{AgentClientId, InstallScope};

const PRODUCT_CLI_PATHS: &[&str] = &[
    "check",
    "audit",
    "gate",
    "gate-config",
    "drift",
    "architecture",
    "policy",
    "export",
    "baseline",
    "audit-chain",
    "l4-validate",
    "validate",
    "mcp install",
    "mcp refresh",
    "mcp serve",
    "mcp-config",
    "plan dashboard",
    "dashboard architecture",
    "dashboard drift",
    "dashboard suppressions",
    "dashboard",
    "watch",
    "intercept",
    "hook",
    "hooks",
    "admin",
    "edda",
    "capsule",
    "insights",
    "kindling",
    "init",
    "start",
    "new",
    "wizard",
    "",
    "welcome",
    "admin auth",
    "auth",
    "auth login",
    "login",
    "auth logout",
    "logout",
    "auth whoami",
    "whoami",
    "auth refresh",
    "config",
    "migrate",
    "update",
    "uninstall",
    "doctor",
    "version",
    "licenses",
    "tutorial",
    "workspace",
    "report-fp",
    "ember",
    "exception",
    "status",
    "impact",
    "telemetry",
    "lsp",
    "skill install",
    "gctx egress",
    "mcp pin",
    "mcp unpin",
    "dashboard --web",
];

const INTERNAL_CLI_PATHS: &[&str] = &["graph-base"];
// Reviewed nested package boundaries and leaves that share the owning projected
// delivery's shipping and gating posture. Keep this narrow: an unlisted sibling
// must fail the recursive inventory.
const COLLAPSED_CLI_PREFIXES: &[&str] = &[
    "admin activity",
    "admin approve",
    "admin audit",
    "admin auth",
    "admin email-send",
    "admin email-update",
    "admin fleet",
    "admin invite",
    "admin list",
    "admin name-update",
    "admin revoke",
    "admin send-migration",
    "admin show",
    "admin users",
    "architecture show",
    "architecture validate",
    "baseline verify",
    "capsule create",
    "capsule explain",
    "capsule prune",
    "capsule verify",
    "config convert",
    "config set",
    "config show",
    "drift compare",
    "drift list",
    "drift migrate",
    "drift report",
    "drift snapshot",
    "edda list",
    "edda ls",
    "edda show",
    "ember list",
    "ember ls",
    "exception grant",
    "exception list",
    "exception migrate",
    "exception revoke",
    "exception show",
    "exception verify",
    "gctx egress",
    "hook bootstrap",
    "hook post-commit",
    "hook post-merge",
    "hook post-rewrite",
    "hook pre-commit",
    "hook pre-push",
    "hooks install",
    "hooks status",
    "hooks uninstall",
    "intercept start",
    "intercept status",
    "intercept stop",
    "intercept unblock",
    "kindling usage",
    "migrate architecture",
    "migrate format",
    "migrate gate-config",
    "migrate schema",
    "policy attack-regression",
    "policy diff",
    "policy eval",
    "policy eval-regression",
    "policy explain",
    "policy install",
    "policy list",
    "policy members",
    "policy probe-trends",
    "policy show",
    "policy test",
    "policy validate",
    "telemetry off",
    "telemetry on",
    "telemetry reset-id",
    "workspace allow",
    "workspace deny",
    "workspace install-hook",
    "workspace list",
    "workspace mode",
    "workspace register",
    "workspace unregister",
];
// graph-base is the one internal CLI delivery; its two operations are reviewed
// separately so a new sibling cannot inherit the exclusion silently.
const COLLAPSED_INTERNAL_CLI_PREFIXES: &[&str] = &["graph-base build", "graph-base gc"];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Locator {
    kind: String,
    parts: Vec<String>,
}

impl Locator {
    fn new(kind: &str, parts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            kind: kind.to_owned(),
            parts: parts.into_iter().map(Into::into).collect(),
        }
    }
}

#[test]
fn active_catalogue_exactly_matches_rust_host_projections() {
    let catalogue: Value = serde_json::from_str(include_str!("../../../flags/surfaces.json"))
        .expect("canonical flags/surfaces.json parses");

    validate_active_exclusions(&catalogue);

    let (cli_product, cli_internal) = cli_projection();
    let (daemon_product, daemon_internal) = daemon_projection();

    let hosts = [
        (
            "cli",
            catalogue_sets(&catalogue, "cli", |_| true),
            (cli_product, cli_internal),
        ),
        (
            "mcp-tool",
            catalogue_sets(&catalogue, "mcp-tool", |_| true),
            (mcp_tools_projection(), BTreeSet::new()),
        ),
        (
            "mcp-resource",
            catalogue_sets(&catalogue, "mcp-resource", |_| true),
            (mcp_resources_projection(), BTreeSet::new()),
        ),
        (
            "daemon-rpc",
            catalogue_sets(&catalogue, "daemon-rpc", |_| true),
            (daemon_product, daemon_internal),
        ),
        (
            "dashboard-server",
            catalogue_sets(&catalogue, "dashboard-route", is_dashboard_server_locator),
            (dashboard_server_projection(), BTreeSet::new()),
        ),
        (
            "hook",
            catalogue_sets(&catalogue, "hook", |_| true),
            (hook_projection(), BTreeSet::new()),
        ),
        (
            "integration",
            catalogue_sets(&catalogue, "integration", |_| true),
            (integration_projection(), BTreeSet::new()),
        ),
    ];

    let failures: Vec<String> = hosts
        .into_iter()
        .filter_map(|(host, catalogue, projected)| compare_host(host, catalogue, projected))
        .collect();

    assert!(
        failures.is_empty(),
        "Rust host completeness failed:\n{}",
        failures.join("\n")
    );
}

fn catalogue_sets(
    catalogue: &Value,
    kind: &str,
    include: fn(&Locator) -> bool,
) -> (BTreeSet<Locator>, BTreeSet<Locator>) {
    (
        catalogue_set(catalogue, "deliverySurfaces", "product", kind, include),
        catalogue_set(
            catalogue,
            "excludedDeliverySurfaces",
            "internal",
            kind,
            include,
        ),
    )
}

fn catalogue_set(
    catalogue: &Value,
    collection: &str,
    set_name: &str,
    kind: &str,
    include: fn(&Locator) -> bool,
) -> BTreeSet<Locator> {
    let mut result = BTreeSet::new();
    for surface in catalogue[collection]
        .as_array()
        .unwrap_or_else(|| panic!("{collection} must be an array"))
        .iter()
        .filter(|surface| surface["status"].as_str() == Some("active"))
    {
        let locator = &surface["locator"];
        if locator["kind"].as_str() != Some(kind) {
            continue;
        }
        let parsed = parse_locator(locator);
        if !include(&parsed) {
            continue;
        }
        assert!(
            result.insert(parsed.clone()),
            "catalogue {set_name} {kind} has duplicate locator {parsed:?}"
        );
    }
    result
}

fn parse_locator(locator: &Value) -> Locator {
    let kind = required_str(locator, "kind");
    match kind {
        "cli" => Locator::new(
            kind,
            locator["commandPath"]
                .as_array()
                .expect("CLI commandPath is an array")
                .iter()
                .map(|part| part.as_str().expect("CLI commandPath part is a string")),
        ),
        "mcp-tool" => Locator::new(kind, [required_str(locator, "name")]),
        "mcp-resource" => Locator::new(kind, [required_str(locator, "uri")]),
        "daemon-rpc" => Locator::new(kind, [required_str(locator, "method")]),
        "dashboard-route" => Locator::new(kind, [required_str(locator, "path")]),
        "hook" => Locator::new(kind, [required_str(locator, "hook")]),
        "integration" => Locator::new(
            kind,
            [
                required_str(locator, "integrationId"),
                required_str(locator, "capability"),
            ],
        ),
        other => panic!("unsupported Rust host locator kind {other}"),
    }
}

fn required_str<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field]
        .as_str()
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| panic!("{field} must be a non-empty string in {value}"))
}

fn validate_active_exclusions(catalogue: &Value) {
    for surface in catalogue["excludedDeliverySurfaces"]
        .as_array()
        .expect("excludedDeliverySurfaces must be an array")
        .iter()
        .filter(|surface| surface["status"].as_str() == Some("active"))
    {
        for field in ["owner", "reason", "reviewReference"] {
            required_str(surface, field);
        }
        assert_eq!(
            surface["classification"].as_str(),
            Some("internal-plumbing"),
            "active exclusion must be classified as internal-plumbing: {surface}"
        );
    }
}

fn cli_projection() -> (BTreeSet<Locator>, BTreeSet<Locator>) {
    let command = Cli::command();
    assert_cli_command_paths_are_projected(
        &command,
        PRODUCT_CLI_PATHS,
        INTERNAL_CLI_PATHS,
        COLLAPSED_CLI_PREFIXES,
        COLLAPSED_INTERNAL_CLI_PREFIXES,
    );
    (
        cli_projection_set(&command, PRODUCT_CLI_PATHS, "product"),
        cli_projection_set(&command, INTERNAL_CLI_PATHS, "internal"),
    )
}

fn assert_cli_command_paths_are_projected(
    command: &Command,
    product_paths: &[&str],
    internal_paths: &[&str],
    collapsed_product_prefixes: &[&str],
    collapsed_internal_prefixes: &[&str],
) {
    let uncovered = uncovered_cli_command_paths(
        command,
        product_paths,
        internal_paths,
        collapsed_product_prefixes,
        collapsed_internal_prefixes,
    );

    assert!(
        uncovered.is_empty(),
        "Clap command paths and aliases missing from CLI projections: {uncovered:?}"
    );
}

fn uncovered_cli_command_paths(
    command: &Command,
    product_paths: &[&str],
    internal_paths: &[&str],
    collapsed_product_prefixes: &[&str],
    collapsed_internal_prefixes: &[&str],
) -> Vec<Vec<String>> {
    let product: BTreeSet<Vec<String>> = product_paths
        .iter()
        .map(|path| path.split_ascii_whitespace().map(str::to_owned).collect())
        .collect();
    let internal: BTreeSet<Vec<String>> = internal_paths
        .iter()
        .map(|path| path.split_ascii_whitespace().map(str::to_owned).collect())
        .collect();
    let projected: BTreeSet<Vec<String>> = product
        .iter()
        .cloned()
        .chain(internal.iter().cloned())
        .collect();
    let collapsed_product: BTreeSet<Vec<String>> = collapsed_product_prefixes
        .iter()
        .map(|path| path.split_ascii_whitespace().map(str::to_owned).collect())
        .collect();
    let collapsed_internal: BTreeSet<Vec<String>> = collapsed_internal_prefixes
        .iter()
        .map(|path| path.split_ascii_whitespace().map(str::to_owned).collect())
        .collect();
    let mut clap_paths = clap_command_paths(command);
    for path in &projected {
        clap_paths.extend(projected_argument_spellings(command, path));
    }
    for prefix in &collapsed_product {
        assert!(
            product
                .iter()
                .any(|boundary| !boundary.is_empty() && prefix.starts_with(boundary)),
            "collapsed CLI prefix must be beneath a product projection: {prefix:?}"
        );
        assert!(
            clap_paths.contains(prefix),
            "collapsed CLI prefix must exist in Clap: {prefix:?}"
        );
    }
    for prefix in &collapsed_internal {
        assert!(
            internal
                .iter()
                .any(|boundary| !boundary.is_empty() && prefix.starts_with(boundary)),
            "collapsed internal CLI prefix must be beneath an internal projection: {prefix:?}"
        );
        assert!(
            clap_paths.contains(prefix),
            "collapsed internal CLI prefix must exist in Clap: {prefix:?}"
        );
    }

    clap_paths
        .into_iter()
        .filter(|path| {
            !projected.contains(path)
                && !projected
                    .iter()
                    .any(|candidate| candidate.starts_with(path))
                && !collapsed_product
                    .iter()
                    .chain(&collapsed_internal)
                    .any(|prefix| path.starts_with(prefix))
        })
        .collect()
}

fn clap_command_paths(command: &Command) -> BTreeSet<Vec<String>> {
    fn visit(command: &Command, parent: &[String], paths: &mut BTreeSet<Vec<String>>) {
        for child in command.get_subcommands() {
            for spelling in clap_subcommand_spellings(child) {
                let mut path = parent.to_vec();
                path.push(spelling);
                assert!(
                    paths.insert(path.clone()),
                    "duplicate Clap command path or alias: {path:?}"
                );
                visit(child, &path, paths);
            }
        }
    }

    let mut paths = BTreeSet::new();
    visit(command, &[], &mut paths);
    paths
}

fn clap_subcommand_spellings(command: &Command) -> BTreeSet<String> {
    let mut spellings = BTreeSet::from([command.get_name().to_owned()]);
    spellings.extend(command.get_all_aliases().map(str::to_owned));
    spellings.extend(command.get_long_flag().map(|flag| format!("--{flag}")));
    spellings.extend(
        command
            .get_all_long_flag_aliases()
            .map(|flag| format!("--{flag}")),
    );
    spellings.extend(command.get_short_flag().map(|flag| format!("-{flag}")));
    spellings.extend(
        command
            .get_all_short_flag_aliases()
            .map(|flag| format!("-{flag}")),
    );
    spellings
}

fn clap_argument_spellings(argument: &Arg) -> BTreeSet<String> {
    let mut spellings = BTreeSet::new();
    spellings.extend(argument.get_long().map(|flag| format!("--{flag}")));
    spellings.extend(
        argument
            .get_all_aliases()
            .unwrap_or_default()
            .into_iter()
            .map(|flag| format!("--{flag}")),
    );
    spellings.extend(argument.get_short().map(|flag| format!("-{flag}")));
    spellings.extend(
        argument
            .get_all_short_aliases()
            .unwrap_or_default()
            .into_iter()
            .map(|flag| format!("-{flag}")),
    );
    spellings
}

fn subcommand_for_spelling<'a>(command: &'a Command, spelling: &str) -> Option<&'a Command> {
    command
        .get_subcommands()
        .find(|candidate| clap_subcommand_spellings(candidate).contains(spelling))
}

fn projected_argument_spellings(command: &Command, path: &[String]) -> BTreeSet<Vec<String>> {
    let Some((terminal, parent)) = path.split_last() else {
        return BTreeSet::new();
    };
    let mut current = command;
    for spelling in parent {
        let Some(next) = subcommand_for_spelling(current, spelling) else {
            return BTreeSet::new();
        };
        current = next;
    }
    let Some(argument) = current
        .get_arguments()
        .find(|argument| clap_argument_spellings(argument).contains(terminal))
    else {
        return BTreeSet::new();
    };

    clap_argument_spellings(argument)
        .into_iter()
        .map(|spelling| {
            let mut alias_path = parent.to_vec();
            alias_path.push(spelling);
            alias_path
        })
        .collect()
}

#[test]
fn recursive_cli_inventory_rejects_unprojected_nested_commands_and_aliases() {
    let command = Command::new("test")
        .subcommand(Command::new("mcp").subcommand(Command::new("future").alias("f")));
    let uncovered = uncovered_cli_command_paths(&command, &["mcp install"], &[], &[], &[]);

    assert_eq!(
        uncovered,
        [
            vec!["mcp".to_owned(), "f".to_owned()],
            vec!["mcp".to_owned(), "future".to_owned()],
        ]
    );
}

#[test]
fn recursive_cli_inventory_rejects_unprojected_flag_style_subcommand_aliases() {
    let command = Command::new("test").subcommand(
        Command::new("check")
            .long_flag("check-now")
            .long_flag_alias("verify-now")
            .short_flag('c')
            .short_flag_alias('v'),
    );
    let uncovered = uncovered_cli_command_paths(&command, &["check"], &[], &[], &[]);

    assert_eq!(
        uncovered,
        [
            vec!["--check-now".to_owned()],
            vec!["--verify-now".to_owned()],
            vec!["-c".to_owned()],
            vec!["-v".to_owned()],
        ]
    );
}

#[test]
fn recursive_cli_inventory_rejects_aliases_of_projected_arguments() {
    let command = Command::new("test").subcommand(
        Command::new("dashboard").arg(
            Arg::new("web")
                .long("web")
                .alias("browser")
                .short('w')
                .short_alias('b'),
        ),
    );
    let uncovered =
        uncovered_cli_command_paths(&command, &["dashboard", "dashboard --web"], &[], &[], &[]);

    assert_eq!(
        uncovered,
        [
            vec!["dashboard".to_owned(), "--browser".to_owned()],
            vec!["dashboard".to_owned(), "-b".to_owned()],
            vec!["dashboard".to_owned(), "-w".to_owned()],
        ]
    );
}

#[test]
fn recursive_cli_inventory_does_not_broaden_a_projected_parent_boundary() {
    let command = Command::new("test").subcommand(
        Command::new("admin")
            .subcommand(Command::new("auth").subcommand(Command::new("status")))
            .subcommand(Command::new("future")),
    );
    let uncovered = uncovered_cli_command_paths(
        &command,
        &["admin", "admin auth"],
        &[],
        &["admin auth"],
        &[],
    );

    assert_eq!(uncovered, [vec!["admin".to_owned(), "future".to_owned()]]);
}

fn cli_projection_set(command: &Command, paths: &[&str], set_name: &str) -> BTreeSet<Locator> {
    let mut projected = BTreeSet::new();
    for raw_path in paths {
        let parts: Vec<String> = raw_path
            .split_ascii_whitespace()
            .map(str::to_owned)
            .collect();
        assert_cli_path_exists(command, &parts);
        let locator = Locator::new("cli", parts);
        assert!(
            projected.insert(locator.clone()),
            "CLI {set_name} projection has duplicate locator {locator:?}"
        );
    }
    projected
}

fn assert_cli_path_exists(command: &Command, path: &[String]) {
    let mut current = command;
    for (index, part) in path.iter().enumerate() {
        if let Some(next) = subcommand_for_spelling(current, part) {
            current = next;
            continue;
        }
        if part.starts_with('-') {
            assert_eq!(
                index + 1,
                path.len(),
                "CLI option must terminate projected path {path:?}"
            );
            assert!(
                current
                    .get_arguments()
                    .any(|argument| clap_argument_spellings(argument).contains(part)),
                "projected CLI option path does not exist in Clap: {path:?}"
            );
            return;
        }
        if current.get_arguments().any(|argument| {
            argument.get_index().is_some()
                || (argument.get_long().is_none() && argument.get_short().is_none())
        }) {
            assert_eq!(
                index + 1,
                path.len(),
                "CLI positional value must terminate projected path {path:?}"
            );
            return;
        }
        current = current
            .get_subcommands()
            .find(|candidate| candidate.get_name() == part)
            .unwrap_or_else(|| {
                panic!("projected CLI command path does not exist in Clap: {path:?}")
            });
    }
}

fn mcp_tools_projection() -> BTreeSet<Locator> {
    unique_host_set(
        "mcp-tool",
        "product",
        crate::mcp::tools::registry::all()
            .iter()
            .map(|tool| Locator::new("mcp-tool", [tool.name])),
    )
}

fn mcp_resources_projection() -> BTreeSet<Locator> {
    unique_host_set(
        "mcp-resource",
        "product",
        crate::mcp::resources::list().into_iter().map(|descriptor| {
            Locator::new(
                "mcp-resource",
                [required_str(&descriptor, "uri").to_owned()],
            )
        }),
    )
}

fn daemon_projection() -> (BTreeSet<Locator>, BTreeSet<Locator>) {
    let product = CANONICAL_GOVERNED_METHODS
        .iter()
        .filter(|entry| entry.visibility == DeliveryVisibility::Product)
        .map(|entry| Locator::new("daemon-rpc", [entry.method]));
    let internal = CANONICAL_GOVERNED_METHODS
        .iter()
        .filter(|entry| entry.visibility == DeliveryVisibility::Internal)
        .map(|entry| Locator::new("daemon-rpc", [entry.method]));
    (
        unique_host_set("daemon-rpc", "product", product),
        unique_host_set("daemon-rpc", "internal", internal),
    )
}

fn dashboard_server_projection() -> BTreeSet<Locator> {
    let runtime = unique_host_set(
        "dashboard-server",
        "product",
        anvil_dashboard_server::dashboard_route_paths()
            .map(|path| Locator::new("dashboard-route", [path])),
    );
    let document = anvil_dashboard_server::openapi_document();
    let openapi = unique_host_set(
        "dashboard-server",
        "OpenAPI",
        document["paths"]
            .as_object()
            .expect("dashboard OpenAPI paths is an object")
            .keys()
            .map(|path| Locator::new("dashboard-route", [path.as_str()])),
    );
    assert_eq!(
        runtime, openapi,
        "dashboard runtime routes and OpenAPI paths must match exactly"
    );
    runtime
}

fn is_dashboard_server_locator(locator: &Locator) -> bool {
    let path = locator
        .parts
        .first()
        .map(String::as_str)
        .unwrap_or_default();
    path == "/healthz" || path == "/openapi.json" || path.starts_with("/api/v1/")
}

fn hook_projection() -> BTreeSet<Locator> {
    unique_host_set(
        "hook",
        "product",
        HookKind::ALL
            .iter()
            .map(|kind| Locator::new("hook", [kind.filename()])),
    )
}

fn integration_projection() -> BTreeSet<Locator> {
    let mut locators = Vec::new();
    for client in AgentClientId::all() {
        if client.supports_mcp(InstallScope::Global) || client.supports_mcp(InstallScope::Project) {
            locators.push(Locator::new("integration", [client.label(), "mcp"]));
        }
        if client.supports_skill(InstallScope::Global)
            || client.supports_skill(InstallScope::Project)
        {
            locators.push(Locator::new("integration", [client.label(), "skill"]));
        }
    }
    unique_host_set("integration", "product", locators)
}

fn unique_host_set(
    host: &str,
    set_name: &str,
    locators: impl IntoIterator<Item = Locator>,
) -> BTreeSet<Locator> {
    let mut result = BTreeSet::new();
    for locator in locators {
        assert!(
            result.insert(locator.clone()),
            "{host} {set_name} projection has duplicate locator {locator:?}"
        );
    }
    result
}

fn compare_host(
    host: &str,
    catalogue: (BTreeSet<Locator>, BTreeSet<Locator>),
    projected: (BTreeSet<Locator>, BTreeSet<Locator>),
) -> Option<String> {
    let (catalogue_product, catalogue_internal) = catalogue;
    let (projected_product, projected_internal) = projected;
    let overlap: Vec<_> = catalogue_product
        .intersection(&catalogue_internal)
        .cloned()
        .collect();
    let missing_product_from_host: Vec<_> = catalogue_product
        .difference(&projected_product)
        .cloned()
        .collect();
    let missing_product_from_catalogue: Vec<_> = projected_product
        .difference(&catalogue_product)
        .cloned()
        .collect();
    let missing_internal_from_host: Vec<_> = catalogue_internal
        .difference(&projected_internal)
        .cloned()
        .collect();
    let missing_internal_from_catalogue: Vec<_> = projected_internal
        .difference(&catalogue_internal)
        .cloned()
        .collect();

    if overlap.is_empty()
        && missing_product_from_host.is_empty()
        && missing_product_from_catalogue.is_empty()
        && missing_internal_from_host.is_empty()
        && missing_internal_from_catalogue.is_empty()
    {
        return None;
    }

    Some(format!(
        "{host}: overlap={overlap:?}; product missing_from_host={missing_product_from_host:?}; product missing_from_catalogue={missing_product_from_catalogue:?}; internal missing_from_host={missing_internal_from_host:?}; internal missing_from_catalogue={missing_internal_from_catalogue:?}"
    ))
}
