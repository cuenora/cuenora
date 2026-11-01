//! Invoke once from an external scheduler; no HTTP server or embedded timer.
use cuenora_api::{config::pool_from_url, recurrence_job};
use std::process::ExitCode;

const HELP: &str = "Cuenora recurrence job\n\nUsage: cuenora-recurrence-job [--help|-h]\n\nRun once; schedule daily externally. Requires exported CUENORA_DATABASE_URL\nand migrations through 0018. Does not load .env or post financial transactions.\nAn overlapping run is a successful no-op. Output contains aggregate counts only.\nExit codes: 0 completed/overlap/help; 1 job or per-rule failure; 2 arguments/config.\n";

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if !args.is_empty() {
        eprintln!("Invalid arguments. Use --help.");
        return ExitCode::from(2);
    }
    let Ok(url) = std::env::var("CUENORA_DATABASE_URL") else {
        eprintln!("Invalid or missing CUENORA_DATABASE_URL.");
        return ExitCode::from(2);
    };
    let Ok(Some(pool)) = pool_from_url(Some(&url)) else {
        eprintln!("Invalid or missing CUENORA_DATABASE_URL.");
        return ExitCode::from(2);
    };
    let result = recurrence_job::run(&pool, || std::time::SystemTime::now().into()).await;
    pool.close().await;
    match result {
        Ok(report) => {
            if report.already_running {
                println!("status=already_running");
            } else {
                println!(
                    "status=completed rules_processed={} occurrences_created={} rules_failed={}",
                    report.processed, report.generated, report.failed
                );
            }
            if report.failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(_) => {
            eprintln!("Recurrence job unavailable; retry safely.");
            ExitCode::FAILURE
        }
    }
}
