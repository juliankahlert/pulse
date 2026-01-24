use anyhow::{Result, anyhow};

use crate::clrs::Clrs;
use crate::config::Config;
use owo_colors::OwoColorize;

/// Get the current username
pub fn get_username() -> Result<String> {
    users::get_current_username()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to get username"))
}

/// Get the current working directory, with home directory abbreviated as ~
pub fn get_current_directory() -> Result<String> {
    let cwd = std::env::current_dir()?;
    let home = dirs::home_dir().unwrap_or_default();

    let path_str = cwd.to_string_lossy();

    if let Some(home_str) = home.to_str()
        && let Some(relative) = path_str.strip_prefix(home_str)
    {
        let relative = relative.strip_prefix('/').unwrap_or(relative);
        if relative.is_empty() {
            return Ok("~".to_string());
        } else {
            return Ok(format!("~/{}", relative));
        }
    }

    Ok(path_str.to_string())
}

/// Generate the prompt string based on configuration
pub fn generate_prompt(config: &Config) -> Result<String> {
    let mode = config.mode.as_deref().unwrap_or("DualLine");

    let user = get_username()?;
    let host = get_hostname()?;
    let dir = get_current_directory()?;
    let git_repo = get_git_repo_name();
    let exit_code = get_exit_code();

    let user_color = config.get_color("username").to_dyn();
    let host_color = config.get_color("hostname").to_dyn();
    let dir_color = config.get_color("current_directory").to_dyn();
    let git_color = config.get_color("git_branch").to_dyn();
    let white = Clrs::White.to_dyn();

    let mut first_line = String::new();
    if let Some(repo_name) = git_repo {
        // Git mode
        let repo_root = std::fs::canonicalize(
            gix::discover(".")?
                .work_dir()
                .ok_or(anyhow!("no work dir"))?,
        )?;
        let current = std::env::current_dir()?;
        let relative = current.strip_prefix(&repo_root).unwrap_or(&current);
        let relative_str = relative.to_string_lossy();
        let parts: Vec<&str> = relative_str.split('/').filter(|s| !s.is_empty()).collect();
        let branch = get_git_branch().unwrap_or_else(|| "unknown".to_string());
        let nav = truncate_git_path(&parts);
        first_line.push_str(&format!("{}", user.color(user_color)));
        first_line.push_str(&format!("{}", "@".color(white)));
        first_line.push_str(&format!("{}", host.color(host_color)));
        first_line.push_str(&format!("{}", ": [".color(white)));
        first_line.push_str(&format!("{}", repo_name.color(git_color)));
        first_line.push_str(&format!("{}", " : ".color(white)));
        first_line.push_str(&format!("{}", branch.color(git_color)));
        first_line.push_str(&format!("{}", "] ".color(white)));
        first_line.push_str(&format!("{}", nav.color(dir_color)));
    } else {
        // Non-git mode
        let (root, nav) = if dir.starts_with("~/") {
            ("~", dir.strip_prefix("~/").unwrap_or(&dir).to_string())
        } else {
            ("/", dir.strip_prefix("/").unwrap_or(&dir).to_string())
        };
        let nav_parts: Vec<&str> = nav.split('/').filter(|s| !s.is_empty()).collect();
        let path_display = truncate_non_git_path(root, &nav_parts, mode == "Inline");
        first_line.push_str(&format!("{}", user.color(user_color)));
        first_line.push_str(&format!("{}", "@".color(white)));
        first_line.push_str(&format!("{}", host.color(host_color)));
        first_line.push_str(&format!("{}", ":".color(white)));
        first_line.push_str(&format!("{}", path_display.color(dir_color)));
    }

    let prompt = if mode == "Inline" {
        format!("{} $ ", first_line)
    } else {
        format!("{}\n└─ {} $ ", first_line, exit_code)
    };
    Ok(prompt)
}

/// Get the system's hostname
pub fn get_hostname() -> Result<String> {
    hostname::get()
        .map(|s| s.to_string_lossy().to_string())
        .map_err(|e| anyhow::anyhow!("Unable to get hostname: {}", e))
}

/// Get the current Git branch if in a repository
pub fn get_git_branch() -> Option<String> {
    let repo = gix::discover(".").ok()?;
    let mut head = repo.head().ok()?;
    if head.is_detached() {
        head.try_peel_to_id_in_place()
            .ok()
            .flatten()
            .map(|id| id.to_hex_with_len(7).to_string())
    } else {
        head.referent_name().map(|name| name.shorten().to_string())
    }
}

/// Get the exit code from environment
pub fn get_exit_code() -> String {
    std::env::var("PIPESTATUS")
        .or_else(|_| std::env::var("LAST_EXIT_CODE"))
        .unwrap_or_else(|_| "0".to_string())
}

/// Get the git repository name if in a repository
pub fn get_git_repo_name() -> Option<String> {
    let repo = gix::discover(".").ok()?;
    let workdir = repo.work_dir()?;
    std::fs::canonicalize(workdir)
        .ok()?
        .file_name()?
        .to_str()
        .map(|s| s.to_string())
}

/// Truncate git path for display
pub fn truncate_git_path(parts: &[&str]) -> String {
    if parts.is_empty() {
        String::new()
    } else if parts.len() > 3 {
        format!("… {}", parts[parts.len() - 3..].join(" › "))
    } else {
        parts.join(" › ")
    }
}

/// Truncate non-git path for display
pub fn truncate_non_git_path(root: &str, parts: &[&str], inline: bool) -> String {
    if inline || parts.is_empty() {
        format!("{} pulse", root)
    } else if parts.len() > 3 {
        format!("{} … {}", root, parts[parts.len() - 3..].join(" › "))
    } else {
        format!("{} {}", root, parts.join(" › "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_username() {
        let username = get_username();
        assert!(username.is_ok());
        let uname = username.unwrap();
        assert!(!uname.is_empty());
    }

    #[test]
    fn test_get_current_directory() {
        let cwd = get_current_directory();
        assert!(cwd.is_ok());
        let cwd_str = cwd.unwrap();
        assert!(!cwd_str.is_empty());
        // Should start with / or ~
        assert!(cwd_str.starts_with('/') || cwd_str.starts_with('~'));
    }

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(hostname.is_ok());
        let hname = hostname.unwrap();
        assert!(!hname.is_empty());
    }

    #[test]
    fn test_get_git_branch() {
        let branch = get_git_branch();
        // Since this is run in a git repo, should be Some
        assert!(branch.is_some());
        let branch_name = branch.unwrap();
        assert!(!branch_name.is_empty());
    }

    #[test]
    fn test_get_git_repo_name() {
        let repo_name = get_git_repo_name();
        // Since this is run in a git repo, should be Some
        assert!(repo_name.is_some());
        let name = repo_name.unwrap();
        assert_eq!(name, "pulse");
        assert!(!name.is_empty());
    }

    #[test]
    fn test_truncate_git_path_empty() {
        assert_eq!(truncate_git_path(&[]), "");
    }

    #[test]
    fn test_truncate_git_path_three_parts() {
        assert_eq!(
            truncate_git_path(&["src", "main", "rust"]),
            "src › main › rust"
        );
    }

    #[test]
    fn test_truncate_git_path_four_parts() {
        assert_eq!(truncate_git_path(&["a", "b", "c", "d"]), "… b › c › d");
    }

    #[test]
    fn test_truncate_git_path_more_than_four() {
        assert_eq!(truncate_git_path(&["x", "y", "z", "a", "b"]), "… z › a › b");
    }

    #[test]
    fn test_truncate_non_git_path_inline() {
        assert_eq!(truncate_non_git_path("~", &["a", "b"], true), "~ pulse");
    }

    #[test]
    fn test_truncate_non_git_path_dualline_empty() {
        assert_eq!(truncate_non_git_path("/", &[], false), "/ pulse");
    }

    #[test]
    fn test_truncate_non_git_path_dualline_three_parts() {
        assert_eq!(
            truncate_non_git_path("~", &["home", "user", "docs"], false),
            "~ home › user › docs"
        );
    }

    #[test]
    fn test_truncate_non_git_path_dualline_four_parts() {
        assert_eq!(
            truncate_non_git_path("/", &["usr", "local", "bin", "pulse"], false),
            "/ … local › bin › pulse"
        );
    }

    #[test]
    fn test_get_exit_code_default() {
        // Ensure no env vars are set
        unsafe {
            std::env::remove_var("PIPESTATUS");
            std::env::remove_var("LAST_EXIT_CODE");
        }
        assert_eq!(get_exit_code(), "0");
    }

    #[test]
    fn test_get_exit_code_pipestatus() {
        unsafe {
            std::env::set_var("PIPESTATUS", "42");
        }
        assert_eq!(get_exit_code(), "42");
        unsafe {
            std::env::remove_var("PIPESTATUS");
        }
    }

    #[test]
    fn test_get_exit_code_last_exit_code() {
        unsafe {
            std::env::remove_var("PIPESTATUS");
            std::env::set_var("LAST_EXIT_CODE", "1");
        }
        assert_eq!(get_exit_code(), "1");
        unsafe {
            std::env::remove_var("LAST_EXIT_CODE");
        }
    }

    #[test]
    fn test_get_exit_code_precedence() {
        unsafe {
            std::env::set_var("PIPESTATUS", "10");
            std::env::set_var("LAST_EXIT_CODE", "20");
        }
        assert_eq!(get_exit_code(), "10"); // PIPESTATUS takes precedence
        unsafe {
            std::env::remove_var("PIPESTATUS");
            std::env::remove_var("LAST_EXIT_CODE");
        }
    }

    #[test]
    fn test_generate_prompt() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.unwrap();
        assert!(p.contains("$ "));
        assert!(p.lines().count() == 2); // DualLine mode
    }
}
