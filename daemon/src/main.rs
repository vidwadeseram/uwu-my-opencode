mod commander;
mod config;
mod error;
mod installer;
mod server;
mod state;
mod supervisor;
mod tunnel;
mod workspace;

use clap::Parser;
use commander::CommanderState;
use config::{AppConfig, Cli};
use server::{create_router, AppContext};
use state::StateManager;
use supervisor::ProcessSupervisor;
use tracing::info;
use tracing_subscriber::EnvFilter;
use workspace::WorkspaceManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    if let Some(config::Command::Install {
        domain,
        email,
        ttyd_user,
        ttyd_pass,
        install_dir,
        workspace_dir,
        skip_ssl,
    }) = cli.command
    {
        installer::run_install(
            domain,
            email,
            ttyd_user,
            ttyd_pass,
            install_dir,
            workspace_dir,
            skip_ssl,
        );
        return Ok(());
    }

    let config = AppConfig::from_cli(&cli);

    info!(
        host = %config.host,
        port = config.port,
        workspace_root = %config.workspace_root.display(),
        execute_commands = config.execute_commands,
        "starting uwu-daemon"
    );

    let state = StateManager::new(
        config.state_file.clone(),
        config.port_range_start,
        config.port_range_end,
        config.ttyd_port_start,
    );
    state.load().await?;

    let supervisor = ProcessSupervisor::new();

    let workspace_manager = WorkspaceManager::new(config.clone(), supervisor.clone());

    if config.skip_dotfiles_bootstrap {
        info!("skipping linux dotfiles bootstrap");
    } else {
        let bootstrap_dotfiles = workspace_manager.bootstrap_linux_editor_configs().await?;
        for command in bootstrap_dotfiles {
            info!(
                command = %command.command,
                executed = command.executed,
                success = ?command.success,
                "linux dotfiles bootstrap"
            );
        }
    }

    let bootstrap_commands = workspace_manager.bootstrap_tmux_tabs().await?;
    for command in bootstrap_commands {
        info!(
            command = %command.command,
            executed = command.executed,
            success = ?command.success,
            "bootstrap command"
        );
    }

    let ttyd_result = workspace_manager
        .start_ttyd_main(config.ttyd_port_start)
        .await?;
    if let Some(url) = ttyd_result.browser_url {
        info!(url = %url, "ttyd is available");
    }

    let ctx = AppContext {
        config: config.clone(),
        state,
        supervisor: supervisor.clone(),
        commander: CommanderState::new(config.tmux_bin.clone()),
    };

    let router = create_router(ctx);

    let bind_addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!(addr = %bind_addr, "listening");

    let server = axum::serve(listener, router);

    let graceful = server.with_graceful_shutdown(shutdown_signal(supervisor));
    graceful.await?;

    Ok(())
}

async fn shutdown_signal(supervisor: ProcessSupervisor) {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    tokio::select! {
        _ = ctrl_c => info!("received SIGINT"),
        _ = sigterm.recv() => info!("received SIGTERM"),
    }

    info!("shutting down, killing child processes");
    supervisor.kill_all().await;
}
