use anvil_bench::cli_command::{help_text, parse_cli_args, run};

fn main() {
    let parsed = match parse_cli_args(std::env::args_os()) {
        Ok(parsed) => parsed,
        Err(err) => {
            let message = err.to_string();
            if message == help_text() {
                println!("{message}");
                return;
            }
            eprintln!("Error: {message}");
            std::process::exit(2);
        }
    };

    match run(&parsed.config).and_then(|report| report.to_json().map_err(Into::into)) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("Error: {err:#}");
            std::process::exit(1);
        }
    }
}
