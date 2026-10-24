// SPDX-License-Identifier: MPL-2.0

use std::process::ExitCode;

use entrylens::config::{self, Config};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const USAGE: &str = "\
Entrylens application server: rooms, doorways and passages counted by Entrylens sensors, with a
web interface showing the occupancy of the rooms.

Usage: entrylens [OPTIONS]

Options:
  -h, --help     Print this help and exit
  -V, --version  Print the version and exit

Configuration: a TOML file, {config} by default, with the optional keys
listen, database and title, and the [monitoring] section.

Environment:
  ENTRYLENS_CONFIG  Path of the configuration file
  RUST_LOG          Log level, e.g. debug (default info)
  TZ                Time zone of the local days (default: the system's)
";

#[tokio::main]
async fn main() -> ExitCode {
    // No options other than help and version: everything else is in the configuration file.
    if let Some(argument) = std::env::args().nth(1) {
        return match argument.as_str() {
            "-h" | "--help" => {
                print!("{}", USAGE.replace("{config}", config::DEFAULT_PATH));
                ExitCode::SUCCESS
            }
            "-V" | "--version" => {
                println!("entrylens {}", env!("CARGO_PKG_VERSION"));
                ExitCode::SUCCESS
            }
            _ => {
                eprintln!("entrylens: unknown argument '{argument}'; see 'entrylens --help'");
                ExitCode::from(2)
            }
        };
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let config = Config::load()?;
    let database = config.database.display();
    let pool = entrylens::db::connect(&config.database)
        .await
        .map_err(|e| format!("cannot open database {database}: {e}"))?;
    let listener = TcpListener::bind(config.listen)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", config.listen))?;
    tracing::info!("listening on {}, database {database}", config.listen);
    let state = entrylens::AppState {
        pool: pool.clone(),
        monitoring: config.monitoring.clone(),
    };
    axum::serve(listener, entrylens::app(state, &config.title))
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| e.to_string())?;
    pool.close().await;
    tracing::info!("stopped");
    Ok(())
}

/// Completes on SIGTERM (systemd stop) or Ctrl+C; in-flight requests are then finished.
async fn shutdown() {
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate => {}
    }
}
