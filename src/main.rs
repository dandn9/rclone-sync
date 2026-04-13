use std::{collections::HashMap, fmt, path::PathBuf, process::Stdio};

use serde::Deserialize;

const DEFAULT_CONFIG: &str = include_str!("../.rclone-sync.example.toml");

#[derive(Deserialize)]
struct Config {
    folder: String,
    map: HashMap<String, String>,
}
#[derive(Clone, Copy)]
enum Commands {
    Sync,
    Config,
}

impl Commands {
    const ALL: [Commands; 2] = [Commands::Sync, Commands::Config];
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Commands::Sync => write!(f, "sync"),
            Commands::Config => write!(f, "config"),
        }
    }
}

fn help_string(commands_vec: &[String]) -> String {
    format!(
        "Usage: rclone-sync <command>\n\nCommands: {}",
        commands_vec.join(",")
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(debug_assertions))]
    std::panic::set_hook(Box::new(|info| {
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            eprintln!("{s}");
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            eprintln!("{s}");
        } else {
            eprintln!("panic");
        }
    }));

    let commands_vec = Commands::ALL.iter().collect::<Vec<_>>();
    let commands_str_vec = commands_vec
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>();
    let command_arg = std::env::args()
        .nth(1)
        .expect(&help_string(&commands_str_vec));

    let command = commands_vec
        .iter()
        .zip(commands_str_vec.clone())
        .find(|(_, s)| s == &command_arg)
        .map(|(cmd, _)| *cmd)
        .expect(&help_string(&commands_str_vec));

    match *command {
        Commands::Config => open_config_file(),
        Commands::Sync => sync_files(),
    }
}
fn get_config_file_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?;
    let config_file = config_dir.join(".rclone-sync.toml");

    if !config_file.exists() {
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(&config_file, DEFAULT_CONFIG)?;
    }

    Result::Ok(config_file)
}

fn open_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let config_file_path = get_config_file_path()?;
    opener::open(&config_file_path)?;
    Ok(())
}
fn expand_home(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(stripped) = path.strip_prefix("~/") {
        Ok(dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(stripped)
            .to_str()
            .ok_or("Could not convert path to string")?
            .to_string())
    } else {
        Ok(PathBuf::from(path)
            .to_str()
            .ok_or("Could not convert path to string")?
            .to_string())
    }
}

fn sync_files() -> Result<(), Box<dyn std::error::Error>> {
    let config_file_path = get_config_file_path()?;
    let config_file_content = std::fs::read_to_string(&config_file_path)?;
    let config: Config = toml::from_str(&config_file_content)?;
    let upstream_files_child = std::process::Command::new("rclone")
        .args(["lsf", &format!("remote:{}", config.folder)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let out = upstream_files_child.wait_with_output()?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "Failed to list upstream files".to_string()
        } else {
            format!("Failed to list upstream files: {stderr}")
        };
        return Err(message.into());
    }

    let stdout_lsf = String::from_utf8(out.stdout)?;
    let child_files = stdout_lsf
        .lines()
        .filter(|s| s.ends_with(".pdf"))
        .collect::<Vec<_>>();

    let paths = child_files.iter().filter_map(|cloud_path| {
        if let Some(real_path) = config.map.get(*cloud_path) {
            Some((real_path, cloud_path))
        } else {
            None
        }
    });

    let mut copy_children: Vec<std::process::Child> = Vec::new();
    for (real_path, cloud_path) in paths {
        println!("Syncing \"{}\" to \"{}\"", cloud_path, real_path);
        let path = expand_home(real_path)?;
        copy_children.push(
            std::process::Command::new("rclone")
                .args([
                    "copyto",
                    &format!("remote:{}/{}", config.folder, cloud_path),
                    &path,
                ])
                .stderr(Stdio::piped())
                .spawn()?,
        );
    }

    for copy_child in copy_children {
        let out = copy_child.wait_with_output()?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                "Failed to sync file".to_string()
            } else {
                format!("Failed to sync file: {stderr}")
            };
            return Err(message.into());
        }
    }
    Ok(())
}
