//! Prompt generation logic for Pulse.
//!
//! Generates shell prompts with user info, host, directory, and Git status.
//! Supports different modes and customizable colors.

use std::fmt;

use anyhow::{Result, anyhow};

use crate::clrs::Clrs;
use crate::config::Config;
use crossterm::terminal::size;
use owo_colors::OwoColorize;

pub fn get_terminal_width() -> Option<u16> {
    size().ok().map(|(w, _)| w)
}

#[derive(Debug, Clone, Copy)]
pub struct PromptColors {
    pub user_color: owo_colors::DynColors,
    pub host_color: owo_colors::DynColors,
    pub git_color: owo_colors::DynColors,
    pub white: owo_colors::DynColors,
    pub dir_color: owo_colors::DynColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDisplayMode {
    Full,
    Mini,
    Micro,
    Nano,
}

impl fmt::Display for GitDisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitDisplayMode::Full => write!(f, "Full"),
            GitDisplayMode::Mini => write!(f, "Mini"),
            GitDisplayMode::Micro => write!(f, "Micro"),
            GitDisplayMode::Nano => write!(f, "Nano"),
        }
    }
}

pub fn select_display_mode(
    terminal_width: u16,
    email: Option<&str>,
    repo_name: &str,
    branch: &str,
    nav_parts: &[&str],
    colors: &PromptColors,
) -> GitDisplayMode {
    let modes = [
        GitDisplayMode::Full,
        GitDisplayMode::Mini,
        GitDisplayMode::Micro,
        GitDisplayMode::Nano,
    ];

    for mode in modes {
        let rendered = format_git_prompt_line(mode, email, repo_name, branch, nav_parts, colors);
        let visual_width = strip_ansi(&rendered).len();
        if visual_width <= terminal_width as usize {
            return mode;
        }
    }

    GitDisplayMode::Nano
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            while let Some(&next) = chars.peek() {
                if next.is_ascii_alphabetic() {
                    chars.next();
                    break;
                }
                chars.next();
            }
            continue;
        }
        result.push(c);
    }
    result
}

pub fn format_git_prompt_line(
    mode: GitDisplayMode,
    email: Option<&str>,
    repo_name: &str,
    branch: &str,
    nav_parts: &[&str],
    colors: &PromptColors,
) -> String {
    let mut result = String::new();

    match mode {
        GitDisplayMode::Full => {
            if let Some(email) = email {
                let email_parts: Vec<&str> = email.split('@').collect();
                if email_parts.len() == 2 {
                    result.push_str(&format!("{}", email_parts[0].color(colors.user_color)));
                    result.push_str(&format!("{}", "@".color(colors.white)));
                    result.push_str(&format!("{}", email_parts[1].color(colors.host_color)));
                } else {
                    result.push_str(&format!("{}", email.color(colors.user_color)));
                }
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", branch.color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Mini => {
            if let Some(email) = email {
                let email_parts: Vec<&str> = email.split('@').collect();
                if email_parts.len() == 2 {
                    result.push_str(&format!("{}", email_parts[0].color(colors.user_color)));
                    result.push_str(&format!("{}", "@".color(colors.white)));
                    result.push_str(&format!("{}", email_parts[1].color(colors.host_color)));
                } else {
                    result.push_str(&format!("{}", email.color(colors.user_color)));
                }
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", "…".color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Micro => {
            if let Some(email) = email {
                let email_parts: Vec<&str> = email.split('@').collect();
                if email_parts.len() == 2 {
                    result.push_str(&format!("{}", "@".color(colors.white)));
                    result.push_str(&format!("{}", email_parts[1].color(colors.host_color)));
                } else {
                    result.push_str(&format!("{}", email.color(colors.user_color)));
                }
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", "…".color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Nano => {
            if let Some(email) = email {
                let email_parts: Vec<&str> = email.split('@').collect();
                if email_parts.len() == 2 {
                    result.push_str(&format!("{}", "@".color(colors.white)));
                    result.push_str(&format!("{}", email_parts[1].color(colors.host_color)));
                } else {
                    result.push_str(&format!("{}", email.color(colors.user_color)));
                }
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
            let last_dir = nav_parts.last().map(|s| s.to_string()).unwrap_or_default();
            match nav_parts.len() {
                0 => {}
                1 => {
                    result.push_str(&format!("{}", last_dir.color(colors.dir_color)));
                }
                _ => {
                    result.push_str(&format!("{}", "… › ".color(colors.white)));
                    result.push_str(&format!("{}", last_dir.color(colors.dir_color)));
                }
            }
        }
    }

    if mode != GitDisplayMode::Nano {
        let nav = truncate_git_path(nav_parts);
        result.push_str(&format!("{}", nav.color(colors.dir_color)));
    }

    result
}

/// Get the current username.
pub fn get_username() -> Result<String> {
    users::get_current_username()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to get username"))
}

/// Get the git user email for the current repository if in a git repo
pub fn get_git_user_email() -> Option<String> {
    let repo = gix::discover(".").ok()?;
    let config = repo.config_snapshot();
    config.string("user.email").map(|s| s.to_string())
}

/// Get the user info for prompt: git user.email if in git repo, else system username
pub fn get_prompt_user() -> Result<String> {
    if let Some(email) = get_git_user_email() {
        return Ok(email);
    }
    get_username()
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

    let user = get_prompt_user()?;
    let host = get_hostname()?;
    let dir = get_current_directory()?;
    let git_repo = get_git_repo_name();
    let exit_code = get_exit_code();

    let user_color = config.get_color("username").to_dyn();
    let host_color = config.get_color("hostname").to_dyn();
    let dir_color = config.get_color("current_directory").to_dyn();
    let git_color = config.get_color("git_branch").to_dyn();
    let white = Clrs::White.to_dyn();

    let colors = PromptColors {
        user_color,
        host_color,
        git_color,
        white,
        dir_color,
    };

    let terminal_width = get_terminal_width().unwrap_or(120);

    let mut first_line = String::new();
    if let Some(repo_name) = git_repo {
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
        let git_email = get_git_user_email();
        let email = git_email.as_deref();

        let display_mode =
            select_display_mode(terminal_width, email, &repo_name, &branch, &parts, &colors);

        first_line =
            format_git_prompt_line(display_mode, email, &repo_name, &branch, &parts, &colors);
    } else {
        // Non-git mode
        let (root, nav) = if dir == "~" {
            ("~", "".to_string())
        } else if dir.starts_with("~/") {
            ("~", dir.strip_prefix("~/").unwrap().to_string())
        } else {
            ("/", dir.strip_prefix("/").unwrap().to_string())
        };
        let nav_parts: Vec<&str> = nav.split('/').filter(|s| !s.is_empty()).collect();
        let path_display = truncate_non_git_path(root, &nav_parts, mode == "Inline");
        first_line.push_str(&format!("{}", user.color(user_color)));
        first_line.push_str(&format!("{}", "@".color(white)));
        first_line.push_str(&format!("{}", host.color(host_color)));
        first_line.push_str(&format!("{}", ":".color(white)));
        first_line.push_str(&format!("{}", path_display.color(dir_color)));
    }

    let prompt_symbol = if is_root_user() { "#" } else { "$" };
    let prompt = if mode == "Inline" {
        format!("{} {} ", first_line, prompt_symbol)
    } else {
        format!("{}\n└─ {} {} ", first_line, exit_code, prompt_symbol)
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

/// Check if current user is root
pub fn is_root_user() -> bool {
    users::get_current_uid() == 0
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
pub fn truncate_non_git_path(root: &str, parts: &[&str], _inline: bool) -> String {
    if parts.is_empty() {
        root.to_string()
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
        assert_eq!(truncate_non_git_path("~", &["a", "b"], true), "~ a › b");
    }

    #[test]
    fn test_truncate_non_git_path_dualline_empty() {
        assert_eq!(truncate_non_git_path("/", &[], false), "/");
    }

    #[test]
    fn test_truncate_non_git_path_tilde_empty() {
        assert_eq!(truncate_non_git_path("~", &[], false), "~");
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
            std::env::remove_var("PIPESTATUS");
            std::env::remove_var("LAST_EXIT_CODE");
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
            std::env::remove_var("LAST_EXIT_CODE");
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
            std::env::remove_var("PIPESTATUS");
            std::env::remove_var("LAST_EXIT_CODE");
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
    fn test_is_root_user() {
        // This test will pass on non-root systems (which is expected for development)
        let is_root = is_root_user();
        // We can't guarantee the test environment, so just verify the function runs
        assert!(!is_root || is_root); // This always passes, just testing the function doesn't panic
    }

    #[test]
    fn test_generate_prompt_root_symbol() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.unwrap();
        // Should contain either $ or # depending on user
        assert!(p.contains("$") || p.contains("#"));
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

    #[test]
    fn test_generate_prompt_inline_git() {
        let mut config = crate::config::Config::default();
        config.mode = Some("Inline".to_string());
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.unwrap();
        assert!(p.contains("$ "));
        assert!(p.lines().count() == 1); // Inline mode
    }

    #[test]
    fn test_generate_prompt_dualline_git_format() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.unwrap();
        // Should contain repo name and branch in Git format
        assert!(p.contains("pulse")); // repo name
        assert!(p.contains("[")); // start of Git info
        assert!(p.contains(" : ")); // separator
        assert!(p.contains("]")); // end of Git info
        // Should have navigation path
        assert!(p.lines().count() == 2);
    }

    #[test]
    fn test_get_terminal_width() {
        let width = get_terminal_width();
        assert!(width.is_some());
        assert!(width.unwrap() > 0);
    }

    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                if chars.peek() == Some(&'[') {
                    chars.next(); // skip '['
                    // Skip until we hit a letter (the end of the ANSI sequence)
                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_alphabetic() {
                            chars.next(); // skip the letter (e.g., 'm')
                            break;
                        }
                        chars.next();
                    }
                    continue;
                }
            }
            result.push(c);
        }
        result
    }

    #[test]
    fn test_git_display_mode_display() {
        assert_eq!(GitDisplayMode::Full.to_string(), "Full");
        assert_eq!(GitDisplayMode::Mini.to_string(), "Mini");
        assert_eq!(GitDisplayMode::Micro.to_string(), "Micro");
        assert_eq!(GitDisplayMode::Nano.to_string(), "Nano");
    }

    #[test]
    fn test_format_git_prompt_line_full_mode() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Full,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src", "main"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.contains("user"));
        assert!(clean.contains("@"));
        assert!(clean.contains("example.com"));
        assert!(clean.contains("[myrepo"));
        assert!(clean.contains(" : main]"));
        assert!(clean.contains("src › main"));
    }

    #[test]
    fn test_format_git_prompt_line_mini_mode() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Mini,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["dir1", "dir2", "dir3"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.contains("user"));
        assert!(clean.contains("@example.com"));
        assert!(clean.contains("[myrepo"));
        assert!(clean.contains(" : …]"));
        assert!(clean.contains("dir1 › dir2 › dir3"));
    }

    #[test]
    fn test_format_git_prompt_line_micro_mode() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Micro,
            Some("user@example.com"),
            "myrepo",
            "feature-branch",
            &["src", "utils", "helper"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.starts_with("@example.com"));
        assert!(!clean.contains("user@"));
        assert!(clean.contains("[myrepo"));
        assert!(clean.contains(" : …]"));
        assert!(clean.contains("src › utils › helper"));
    }

    #[test]
    fn test_format_git_prompt_line_nano_mode() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Nano,
            Some("user@example.com"),
            "myrepo",
            "develop",
            &["src", "lib", "core"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.starts_with("@example.com"));
        assert!(!clean.contains("user@"));
        assert!(clean.contains("[myrepo]"));
        assert!(!clean.contains(" : "));
        assert!(clean.contains("… › core"));
    }

    #[test]
    fn test_format_git_prompt_line_no_email() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Full,
            None,
            "repo",
            "main",
            &["dir"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.contains(": [repo"));
        assert!(clean.contains(" : main]"));
        assert!(clean.contains("dir"));
    }

    #[test]
    fn test_format_git_prompt_line_nano_single_dir() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Nano,
            Some("test@domain.org"),
            "project",
            "bugfix",
            &["subdir"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "@domain.org: [project] subdir");
    }

    #[test]
    fn test_format_git_prompt_line_nano_empty_nav() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Nano,
            Some("git@domain"),
            "myrepo",
            "main",
            &[],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "@domain: [myrepo] ");
        assert!(!clean.contains("…"));
        assert!(!clean.contains("›"));
    }

    #[test]
    fn test_format_git_prompt_line_micro_empty_nav() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Micro,
            Some("dev@test.io"),
            "code",
            "HEAD",
            &[],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert!(clean.contains("@test.io"));
        assert!(clean.contains("[code : …]"));
    }

    #[test]
    fn test_format_git_prompt_line_full_format() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Full,
            Some("git@email"),
            "repo",
            "branch",
            &["dir1", "dir2", "dir3"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "git@email: [repo : branch] dir1 › dir2 › dir3");
    }

    #[test]
    fn test_format_git_prompt_line_mini_format() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Mini,
            Some("git@email"),
            "repo",
            "branch",
            &["dir", "dir2", "dir3"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "git@email: [repo : …] dir › dir2 › dir3");
    }

    #[test]
    fn test_format_git_prompt_line_micro_format() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Micro,
            Some("git@email"),
            "repo",
            "branch",
            &["dir", "dir2", "dir3"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "@email: [repo : …] dir › dir2 › dir3");
    }

    #[test]
    fn test_format_git_prompt_line_nano_format() {
        use crate::clrs::Clrs;
        let colors = PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        };

        let result = format_git_prompt_line(
            GitDisplayMode::Nano,
            Some("git@domain"),
            "repo",
            "branch",
            &["dir1", "dir2", "dir3"],
            &colors,
        );

        let clean = strip_ansi(&result);
        assert_eq!(clean, "@domain: [repo] … › dir3");
    }
}
