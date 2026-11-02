//! Render the static API contract for an environment without rewriting source files.
use cuenora_api::config::AppConfig;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!(
            "Usage: cuenora-openapi\n\nRender OpenAPI YAML using CUENORA_PUBLIC_API_URL (default http://127.0.0.1:3000).\nReads process environment, never .env. Invalid configuration exits with status 2."
        );
        return ExitCode::SUCCESS;
    }
    if !args.is_empty() {
        eprintln!("Invalid arguments. Use --help.");
        return ExitCode::from(2);
    }
    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Invalid {}", error.0);
            return ExitCode::from(2);
        }
    };
    let template = include_str!("../../../../../contracts/openapi.yaml");
    print!(
        "{}",
        template.replacen(
            "  - url: http://127.0.0.1:3000",
            &format!("  - url: '{}'", config.public_api_url.replace('\'', "''")),
            1
        )
    );
    ExitCode::SUCCESS
}
