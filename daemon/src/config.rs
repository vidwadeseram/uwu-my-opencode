use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "uwu-daemon", about = "Self-hosted AI coding workspace daemon")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    #[arg(long, default_value = "./workspaces")]
    pub workspace_root: PathBuf,

    #[arg(long, default_value = "./state.json")]
    pub state_file: PathBuf,

    #[arg(long, default_value_t = 4100)]
    pub port_range_start: u16,

    #[arg(long, default_value_t = 4999)]
    pub port_range_end: u16,

    #[arg(long, default_value_t = 7681)]
    pub ttyd_port_start: u16,

    #[arg(long, default_value = "admin")]
    pub ttyd_user: String,

    #[arg(long, default_value = "admin")]
    pub ttyd_pass: String,

    #[arg(long)]
    pub tmux_bin: Option<PathBuf>,

    #[arg(long, default_value = "../opencode")]
    pub opencode_repo: PathBuf,

    #[arg(long, default_value = "../oh-my-opencode")]
    pub oh_my_opencode_repo: PathBuf,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    Start,
    Status,
    Install {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        email: Option<String>,
        #[arg(long, default_value = "admin")]
        ttyd_user: String,
        #[arg(long, default_value = "admin")]
        ttyd_pass: String,
        #[arg(long)]
        install_dir: Option<PathBuf>,
        #[arg(long)]
        workspace_dir: Option<PathBuf>,
        #[arg(long)]
        skip_ssl: bool,
    },
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub workspace_root: PathBuf,
    pub state_file: PathBuf,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub ttyd_port_start: u16,
    pub ttyd_user: String,
    pub ttyd_pass: String,
    pub tmux_bin: String,
    pub opencode_repo: PathBuf,
    pub oh_my_opencode_repo: PathBuf,
    pub execute_commands: bool,
}

impl AppConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        let execute_commands = std::env::var("UWU_EXECUTE_COMMANDS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let opencode_repo = if cli.opencode_repo.is_absolute() {
            cli.opencode_repo.clone()
        } else {
            cwd.join(&cli.opencode_repo)
        };
        let oh_my_opencode_repo = if cli.oh_my_opencode_repo.is_absolute() {
            cli.oh_my_opencode_repo.clone()
        } else {
            cwd.join(&cli.oh_my_opencode_repo)
        };

        let tmux_bin = cli
            .tmux_bin
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "tmux".to_string());

        Self {
            host: cli.host.clone(),
            port: cli.port,
            workspace_root: cli.workspace_root.clone(),
            state_file: cli.state_file.clone(),
            port_range_start: cli.port_range_start,
            port_range_end: cli.port_range_end,
            ttyd_port_start: cli.ttyd_port_start,
            ttyd_user: cli.ttyd_user.clone(),
            ttyd_pass: cli.ttyd_pass.clone(),
            tmux_bin,
            opencode_repo,
            oh_my_opencode_repo,
            execute_commands,
        }
    }
}
