use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn prompt(label: &str, default: &str) -> String {
    print!("{} [{}]: ", label, default);
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    let val = buf.trim().to_string();
    if val.is_empty() {
        default.to_string()
    } else {
        val
    }
}

fn run(label: &str, prog: &str, args: &[&str]) -> bool {
    println!("[uwu] {}", label);
    let status = Command::new(prog)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!(
                "[uwu] command failed (exit {}): {} {}",
                s.code().unwrap_or(-1),
                prog,
                args.join(" ")
            );
            false
        }
        Err(e) => {
            eprintln!("[uwu] failed to run {} — {}", prog, e);
            false
        }
    }
}

fn run_sudo(label: &str, args: &[&str]) -> bool {
    println!("[uwu] {}", label);
    let status = Command::new("sudo")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    match status {
        Ok(s) if s.success() => true,
        _ => {
            eprintln!("[uwu] sudo command failed: {}", args.join(" "));
            false
        }
    }
}

fn write_file_sudo(path: &str, content: &str) -> bool {
    let child = Command::new("sudo")
        .args(["tee", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn();
    match child {
        Ok(mut c) => {
            if let Some(ref mut stdin) = c.stdin {
                let _ = stdin.write_all(content.as_bytes());
            }
            c.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

pub fn run_install(
    domain: Option<String>,
    email: Option<String>,
    ttyd_user: String,
    ttyd_pass: String,
    install_dir: Option<PathBuf>,
    workspace_dir: Option<PathBuf>,
    skip_ssl: bool,
) {
    println!();
    println!("uwu-my-opencode installer");
    println!();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());

    let domain = domain.unwrap_or_else(|| prompt("Domain (e.g. code.example.com)", ""));
    if domain.is_empty() {
        eprintln!("[uwu] domain is required");
        std::process::exit(1);
    }

    let email = email.unwrap_or_else(|| prompt("Email for SSL certificate", ""));
    if email.is_empty() {
        eprintln!("[uwu] email is required");
        std::process::exit(1);
    }

    let install_dir =
        install_dir.unwrap_or_else(|| PathBuf::from(format!("{}/uwu-my-opencode", home)));
    let workspace_dir =
        workspace_dir.unwrap_or_else(|| PathBuf::from(format!("{}/workspaces", home)));

    let install_str = install_dir.to_string_lossy().to_string();
    let workspace_str = workspace_dir.to_string_lossy().to_string();

    println!();
    println!("  domain:     {}", domain);
    println!("  email:      {}", email);
    println!("  ttyd auth:  {} / {}", ttyd_user, ttyd_pass);
    println!("  install:    {}", install_str);
    println!("  workspaces: {}", workspace_str);
    println!();

    let confirm = prompt("Proceed?", "Y");
    if confirm != "Y" && confirm != "y" {
        println!("Aborted.");
        return;
    }

    run_sudo("installing system packages", &["apt-get", "update", "-qq"]);
    run_sudo(
        "installing build tools, nginx, certbot, tmux, zsh",
        &[
            "apt-get",
            "install",
            "-y",
            "-qq",
            "git",
            "curl",
            "build-essential",
            "nginx",
            "certbot",
            "python3-certbot-nginx",
            "tmux",
            "zsh",
            "libevent-dev",
            "libncurses-dev",
            "autoconf",
            "automake",
            "pkg-config",
        ],
    );

    let has_ttyd = Command::new("which")
        .arg("ttyd")
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has_ttyd {
        if !run_sudo(
            "installing ttyd",
            &["apt-get", "install", "-y", "-qq", "ttyd"],
        ) {
            let arch = std::process::Command::new("uname")
                .arg("-m")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "x86_64".to_string());
            let url = format!(
                "https://github.com/tsl0922/ttyd/releases/latest/download/ttyd.{}",
                arch
            );
            run_sudo(
                "downloading ttyd binary",
                &["curl", "-fsSL", &url, "-o", "/usr/local/bin/ttyd"],
            );
            run_sudo(
                "making ttyd executable",
                &["chmod", "+x", "/usr/local/bin/ttyd"],
            );
        }
    }

    let has_bun = Command::new("bash")
        .args(["-c", "command -v bun"])
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has_bun {
        run(
            "installing bun",
            "bash",
            &["-c", "curl -fsSL https://bun.sh/install | bash"],
        );
    }

    let has_cargo = Command::new("bash")
        .args(["-c", "command -v cargo"])
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has_cargo {
        run(
            "installing rust",
            "bash",
            &["-c", "curl https://sh.rustup.rs -sSf | sh -s -- -y"],
        );
    }

    let env_path = format!(
        "{}/.bun/bin:{}/.cargo/bin:/usr/local/bin:/usr/bin:/bin",
        home, home
    );

    if install_dir.join(".git").exists() {
        run(
            "pulling latest",
            "git",
            &["-C", &install_str, "pull", "--ff-only"],
        );
    } else {
        run(
            "cloning repo",
            "git",
            &[
                "clone",
                "https://github.com/vidwadeseram/uwu-my-opencode.git",
                &install_str,
            ],
        );
    }
    run(
        "initializing submodules",
        "git",
        &[
            "-C",
            &install_str,
            "submodule",
            "update",
            "--init",
            "--recursive",
        ],
    );

    let tmux_dir = install_dir.join("tmux");
    let tmux_dir_str = tmux_dir.to_string_lossy().to_string();
    let build_prefix = install_dir.join("build").join("tmux");
    let build_prefix_str = format!("--prefix={}", build_prefix.to_string_lossy());

    if !install_dir.join("build/tmux/bin/tmux").exists() {
        let has_configure = tmux_dir.join("configure").exists();
        if !has_configure {
            run(
                "autogen tmux",
                "sh",
                &[&format!("{}/autogen.sh", tmux_dir_str)],
            );
        }
        run(
            "configure tmux",
            "bash",
            &[
                "-c",
                &format!(
                    "cd {} && ./configure {} --disable-utf8proc",
                    tmux_dir_str, build_prefix_str
                ),
            ],
        );
        let nproc = std::process::Command::new("nproc")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "2".to_string());
        run(
            "building tmux",
            "bash",
            &["-c", &format!("cd {} && make -j{}", tmux_dir_str, nproc)],
        );
        run(
            "installing tmux",
            "bash",
            &["-c", &format!("cd {} && make install", tmux_dir_str)],
        );
    }

    let bun_bin = format!("{home}/.bun/bin/bun");
    let opencode_dir = install_dir.join("opencode").to_string_lossy().to_string();
    let omo_dir = install_dir
        .join("oh-my-opencode")
        .to_string_lossy()
        .to_string();
    run(
        "installing opencode deps",
        &bun_bin,
        &["install", "--cwd", &opencode_dir],
    );
    run(
        "installing oh-my-opencode deps",
        &bun_bin,
        &["install", "--cwd", &omo_dir],
    );

    let cargo_bin = format!("{home}/.cargo/bin/cargo");
    let manifest = install_dir
        .join("daemon/Cargo.toml")
        .to_string_lossy()
        .to_string();
    run(
        "building daemon",
        &cargo_bin,
        &["build", "--manifest-path", &manifest, "--release"],
    );

    std::fs::create_dir_all(format!("{}/workspace-1", workspace_str)).ok();
    std::fs::create_dir_all(format!("{}/.config/uwu", home)).ok();

    let tmux_bin = install_dir
        .join("build/tmux/bin/tmux")
        .to_string_lossy()
        .to_string();
    let daemon_bin = install_dir
        .join("daemon/target/release/uwu-daemon")
        .to_string_lossy()
        .to_string();

    let wrapper = format!("{}/scripts/run-daemon.sh", install_str);
    let wrapper_content = format!(
        "#!/usr/bin/env bash\nexport PATH=\"{}\"\nexport UWU_EXECUTE_COMMANDS=true\nexec \"{}\" \\\n  --host 127.0.0.1 \\\n  --port 18080 \\\n  --workspace-root \"{}\" \\\n  --state-file \"{}/.config/uwu/state.json\" \\\n  --ttyd-port-start 7681 \\\n  --tmux-bin \"{}\" \\\n  --opencode-repo \"{}/opencode\" \\\n  --oh-my-opencode-repo \"{}/oh-my-opencode\"\n",
        env_path, daemon_bin, workspace_str, home, tmux_bin, install_str, install_str
    );
    std::fs::create_dir_all(format!("{}/scripts", install_str)).ok();
    std::fs::write(&wrapper, &wrapper_content).ok();
    run("making wrapper executable", "chmod", &["+x", &wrapper]);

    let service = format!(
        "[Unit]\nDescription=uwu-my-opencode daemon\nAfter=network.target\n\n[Service]\nUser={}\nWorkingDirectory={}/daemon\nExecStart={}\nRestart=always\nRestartSec=2\nEnvironment=HOME={}\nEnvironment=PATH={}\n\n[Install]\nWantedBy=multi-user.target\n",
        user, install_str, wrapper, home, env_path
    );
    write_file_sudo("/etc/systemd/system/uwu-daemon.service", &service);
    run_sudo("reloading systemd", &["systemctl", "daemon-reload"]);
    run_sudo("enabling service", &["systemctl", "enable", "uwu-daemon"]);

    let nginx_conf = format!(
        "server {{\n    listen 80;\n    server_name {};\n\n    location / {{\n        proxy_pass http://127.0.0.1:7681;\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header Upgrade $http_upgrade;\n        proxy_set_header Connection \"upgrade\";\n        proxy_read_timeout 86400;\n    }}\n}}\n",
        domain
    );
    write_file_sudo("/etc/nginx/sites-available/uwu-my-opencode", &nginx_conf);
    run_sudo(
        "enabling nginx site",
        &[
            "ln",
            "-sf",
            "/etc/nginx/sites-available/uwu-my-opencode",
            "/etc/nginx/sites-enabled/uwu-my-opencode",
        ],
    );
    run_sudo(
        "removing default site",
        &["rm", "-f", "/etc/nginx/sites-enabled/default"],
    );
    run_sudo("testing nginx", &["nginx", "-t"]);
    run_sudo("restarting nginx", &["systemctl", "restart", "nginx"]);

    if Command::new("which")
        .arg("ufw")
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        run_sudo("allowing SSH", &["ufw", "allow", "OpenSSH"]);
        run_sudo("allowing nginx", &["ufw", "allow", "Nginx Full"]);
        run_sudo("enabling firewall", &["ufw", "--force", "enable"]);
    }

    run_sudo("starting daemon", &["systemctl", "start", "uwu-daemon"]);

    if !skip_ssl {
        println!();
        println!(
            "  Make sure DNS A record for {} points to this server.",
            domain
        );
        let ssl_confirm = prompt("  DNS is ready, continue with SSL?", "Y");
        if ssl_confirm == "Y" || ssl_confirm == "y" {
            run_sudo(
                "requesting SSL certificate",
                &[
                    "certbot",
                    "--nginx",
                    "-d",
                    &domain,
                    "--non-interactive",
                    "--agree-tos",
                    "-m",
                    &email,
                    "--redirect",
                ],
            );
        } else {
            println!(
                "  Skipping SSL. Run later: sudo certbot --nginx -d {}",
                domain
            );
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  uwu-my-opencode is live!");
    println!();
    println!("  URL:      https://{}", domain);
    println!("  Username: {}", ttyd_user);
    println!("  Password: {}", ttyd_pass);
    println!();
    println!("  Manage:");
    println!("    sudo systemctl status uwu-daemon");
    println!("    sudo systemctl restart uwu-daemon");
    println!("    sudo journalctl -u uwu-daemon -f");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
}
