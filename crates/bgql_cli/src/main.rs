//! Main entry point for the Better GraphQL CLI.

use bgql_cli::{Cli, Commands};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bgql=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Handle LSP separately since it needs async
    if matches!(cli.command, Commands::Lsp) {
        bgql_lsp::run_server().await;
        return;
    }

    // Run other commands
    match bgql_cli::run(cli) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
