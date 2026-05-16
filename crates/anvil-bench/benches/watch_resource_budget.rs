use anvil_bench::watch_resource_budget::{WatchResourceBudgetConfig, run};

fn main() {
    let config = WatchResourceBudgetConfig::from_env().expect("watch resource budget config");
    let verdict = run(&config).expect("watch resource budget measurement");
    println!(
        "{}",
        serde_json::to_string_pretty(&verdict).expect("budget verdict serialises")
    );
    if verdict.status.is_fail() {
        std::process::exit(1);
    }
}
