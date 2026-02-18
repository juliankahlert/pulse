//! Prompt generation logic for Pulse.
//!
//! Generates shell prompts with user info, host, directory, and Git status.
//! Supports different modes and customizable colors.

use std::cell::OnceCell;
use std::fmt;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use crate::clrs::Clrs;
use crate::config::Config;
use crossterm::terminal::size;
use owo_colors::OwoColorize;

const DEFAULT_TERM_WIDTH: usize = 120;
const TRUNCATION_THRESHOLD: usize = 3;

pub fn get_terminal_width() -> Option<u16> {
    size().ok().map(|(w, _)| w)
}

pub fn is_in_git_repo() -> bool {
    let mut current = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return false,
    };

    loop {
        if current.join(".git").is_dir() {
            return true;
        }

        if !current.pop() {
            break;
        }
    }

    false
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

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub repo_name: String,
    pub branch: String,
    pub user_email: Option<String>,
    pub work_dir: PathBuf,
}

pub struct LazyGitInfo {
    cached: OnceCell<Option<GitInfo>>,
}

impl LazyGitInfo {
    pub fn new() -> Self {
        Self {
            cached: OnceCell::new(),
        }
    }

    pub fn get(&self) -> Option<&GitInfo> {
        self.cached
            .get_or_init(|| {
                let repo = match gix::discover(".") {
                    Ok(r) => r,
                    Err(_) => return None,
                };
                let work_dir = match repo.work_dir() {
                    Some(w) => w,
                    None => return None,
                };
                let work_dir = match std::fs::canonicalize(work_dir) {
                    Ok(w) => w,
                    Err(_) => return None,
                };
                let repo_name = match work_dir.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => return None,
                };

                let mut head = match repo.head() {
                    Ok(h) => h,
                    Err(_) => return None,
                };
                let branch = if head.is_detached() {
                    head.try_peel_to_id_in_place()
                        .ok()
                        .flatten()
                        .map(|id| id.to_hex_with_len(7).to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    head.referent_name()
                        .map(|name| name.shorten().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                };

                let config = repo.config_snapshot();
                let user_email = config.string("user.email").map(|s| s.to_string());

                Some(GitInfo {
                    repo_name,
                    branch,
                    user_email,
                    work_dir,
                })
            })
            .as_ref()
    }
}

impl Default for LazyGitInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub fn get_git_info() -> LazyGitInfo {
    LazyGitInfo::new()
}

pub fn select_display_mode(
    terminal_width: u16,
    email: Option<&str>,
    repo_name: &str,
    branch: &str,
    nav_parts: &[&str],
    _colors: &PromptColors,
) -> GitDisplayMode {
    let modes = [
        GitDisplayMode::Full,
        GitDisplayMode::Mini,
        GitDisplayMode::Micro,
        GitDisplayMode::Nano,
    ];

    for mode in modes {
        let width = calculate_git_prompt_width(mode, email, repo_name, branch, nav_parts);
        if width <= terminal_width as usize {
            return mode;
        }
    }

    GitDisplayMode::Nano
}

fn visual_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

fn calculate_git_prompt_width(
    mode: GitDisplayMode,
    email: Option<&str>,
    repo_name: &str,
    branch: &str,
    nav_parts: &[&str],
) -> usize {
    let email_width = email.map_or(0, |e| {
        if let Some((user, host)) = e.split_once('@') {
            visual_width(user) + 1 + visual_width(host)
        } else {
            visual_width(e)
        }
    });

    let repo_len = visual_width(repo_name);
    let branch_len = visual_width(branch);

    let nav_width = match mode {
        GitDisplayMode::Full | GitDisplayMode::Mini | GitDisplayMode::Micro => {
            let truncated = truncate_git_path(nav_parts);
            visual_width(&truncated)
        }
        GitDisplayMode::Nano => {
            if nav_parts.is_empty() {
                0
            } else if nav_parts.len() == 1 {
                visual_width(nav_parts[0])
            } else {
                3 + visual_width("› ") + visual_width(nav_parts.last().unwrap_or(&""))
            }
        }
    };

    match mode {
        GitDisplayMode::Full => email_width + 3 + repo_len + 3 + branch_len + 2 + nav_width,
        GitDisplayMode::Mini => email_width + 3 + repo_len + 3 + 1 + 2 + nav_width,
        GitDisplayMode::Micro => {
            let host_len = email.map_or(0, |e| {
                if let Some((_, host)) = e.split_once('@') {
                    visual_width(host)
                } else {
                    visual_width(e)
                }
            });
            1 + host_len + 3 + repo_len + 3 + 1 + 2 + nav_width
        }
        GitDisplayMode::Nano => {
            let host_len = email.map_or(0, |e| {
                if let Some((_, host)) = e.split_once('@') {
                    visual_width(host)
                } else {
                    visual_width(e)
                }
            });
            let last_dir_width = if nav_parts.is_empty() {
                0
            } else if nav_parts.len() == 1 {
                visual_width(nav_parts[0])
            } else {
                3 + visual_width("› ") + visual_width(nav_parts.last().unwrap_or(&""))
            };
            1 + host_len + 3 + repo_len + 2 + last_dir_width
        }
    }
}

fn format_email_parts(email: &str, colors: &PromptColors, show_full: bool) -> String {
    let mut result = String::new();
    let email_parts: Vec<&str> = email.split('@').collect();
    if email_parts.len() == 2 {
        if show_full {
            result.push_str(&format!("{}", email_parts[0].color(colors.user_color)));
        }
        result.push_str(&format!("{}", "@".color(colors.white)));
        result.push_str(&format!("{}", email_parts[1].color(colors.host_color)));
    } else {
        result.push_str(&format!("{}", email.color(colors.user_color)));
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
                result.push_str(&format_email_parts(email, colors, true));
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", branch.color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Mini => {
            if let Some(email) = email {
                result.push_str(&format_email_parts(email, colors, true));
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", "…".color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Micro => {
            if let Some(email) = email {
                result.push_str(&format_email_parts(email, colors, false));
            }
            result.push_str(&format!("{}", ": [".color(colors.white)));
            result.push_str(&format!("{}", repo_name.color(colors.git_color)));
            result.push_str(&format!("{}", " : ".color(colors.white)));
            result.push_str(&format!("{}", "…".color(colors.git_color)));
            result.push_str(&format!("{}", "] ".color(colors.white)));
        }
        GitDisplayMode::Nano => {
            if let Some(email) = email {
                result.push_str(&format_email_parts(email, colors, false));
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

/// Get the current username from the operating system.
///
/// Returns the username of the currently logged-in user by querying
/// the system via the `users` crate.
///
/// # Returns
/// - `Ok(String)` containing the username on success.
/// - `Err` if the username cannot be retrieved from the system.
///
/// # Use Cases
/// Use this function when you need the raw system username for:
/// - System administration tasks
/// - File ownership checks
/// - Any context where you need the actual OS username
///
/// # Example
/// ```ignore
/// let username = get_username()?;
/// println!("Current user: {}", username);
/// ```
pub fn get_username() -> Result<String> {
    users::get_current_username()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| anyhow::anyhow!("Unable to get username"))
}

/// Get the user information for display in the prompt.
///
/// This function wraps [`get_username()`] and is specifically designed
/// for use in shell prompt generation. It returns the same value as
/// `get_username()` - the system username of the current user.
///
/// # Returns
/// - `Ok(String)` containing the username suitable for prompt display.
/// - `Err` if the username cannot be retrieved.
///
/// # When to Use
/// Use `get_prompt_user()` when generating shell prompts, as it clearly
/// communicates the purpose (prompt display) and guarantees the same
/// username that would be shown in a traditional shell prompt.
///
/// Use [`get_username()`] for non-prompt contexts where you need the
/// system username.
///
/// # Note
/// Currently, this function is an alias for [`get_username()`], but this
/// may change in future versions (e.g., to support custom prompt usernames
/// or different display formats).
pub fn get_prompt_user() -> Result<String> {
    get_username()
}

/// Get the current working directory, with home directory abbreviated as ~
pub fn get_current_directory() -> Result<String> {
    let cwd = std::env::current_dir()?;
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Cannot determine home directory"))?;

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
    let in_git = is_in_git_repo();
    let git_info = if in_git {
        Some(LazyGitInfo::new())
    } else {
        None
    };
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

    let terminal_width = get_terminal_width().unwrap_or(DEFAULT_TERM_WIDTH as u16);

    let mut first_line = String::new();
    if let Some(ref lazy_info) = git_info {
        if let Some(info) = lazy_info.get() {
            let current = std::env::current_dir()?;
            let relative = current.strip_prefix(&info.work_dir).unwrap_or(&current);
            let relative_str = relative.to_string_lossy();
            let parts: Vec<&str> = relative_str.split('/').filter(|s| !s.is_empty()).collect();
            let email = info.user_email.as_deref();

            let display_mode = select_display_mode(
                terminal_width,
                email,
                &info.repo_name,
                &info.branch,
                &parts,
                &colors,
            );

            first_line = format_git_prompt_line(
                display_mode,
                email,
                &info.repo_name,
                &info.branch,
                &parts,
                &colors,
            );
        } else {
            first_line.push_str(&build_non_git_path_string(
                &dir, &user, &host, &colors, mode,
            ));
        }
    } else {
        first_line.push_str(&build_non_git_path_string(
            &dir, &user, &host, &colors, mode,
        ));
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    } else if parts.len() > TRUNCATION_THRESHOLD {
        format!("… {}", parts[parts.len() - TRUNCATION_THRESHOLD..].join(" › "))
    } else {
        parts.join(" › ")
    }
}

/// Truncate non-git path for display
/// Parameter reserved for future use - may affect formatting in inline mode
pub fn truncate_non_git_path(root: &str, parts: &[&str], _inline: bool) -> String {
    if parts.is_empty() {
        root.to_string()
    } else if parts.len() > TRUNCATION_THRESHOLD {
        format!("{} … {}", root, parts[parts.len() - TRUNCATION_THRESHOLD..].join(" › "))
    } else {
        format!("{} {}", root, parts.join(" › "))
    }
}

/// Builds the user@host:path string for non-git mode.
///
/// This helper handles:
/// - Path normalization (extracting root ~ or / and navigation portion)
/// - Navigation splitting by '/'
/// - Truncation of long paths
/// - Building the colored user@host:path string
///
/// # Arguments
/// * `dir` - The current directory path
/// * `user` - The username string
/// * `host` - The hostname string
/// * `user_color` - Color for the username
/// * `host_color` - Color for the hostname
/// * `dir_color` - Color for the directory path
/// * `white` - Color for the separator characters (@ and :)
/// * `mode` - Display mode ("Inline" or "DualLine")
///
/// # Arguments
/// * `dir` - The current directory path
/// * `user` - The username string
/// * `host` - The hostname string
/// * `colors` - The PromptColors struct containing color definitions
/// * `mode` - Display mode ("Inline" or "DualLine")
///
/// # Returns
/// A formatted string with the user@host:path components colored
pub fn build_non_git_path_string(
    dir: &str,
    user: &str,
    host: &str,
    colors: &PromptColors,
    mode: &str,
) -> String {
    let (root, nav) = if dir == "~" {
        ("~", "".to_string())
    } else if dir.starts_with("~/") {
        (
            "~",
            dir.strip_prefix("~/")
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
    } else {
        (
            "/",
            dir.strip_prefix("/")
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
    };
    let nav_parts: Vec<&str> = nav.split('/').filter(|s| !s.is_empty()).collect();
    let path_display = truncate_non_git_path(root, &nav_parts, mode == "Inline");

    let mut result = String::new();
    result.push_str(&format!("{}", user.color(colors.user_color)));
    result.push_str(&format!("{}", "@".color(colors.white)));
    result.push_str(&format!("{}", host.color(colors.host_color)));
    result.push_str(&format!("{}", ":".color(colors.white)));
    result.push_str(&format!("{}", path_display.color(colors.dir_color)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_is_in_git_repo() {
        // This is run in a git repo, should return true
        assert!(is_in_git_repo());
    }

    #[test]
    fn test_get_username() {
        let username = get_username();
        assert!(username.is_ok());
        let uname = username.expect("username should be Ok after is_ok check");
        assert!(!uname.is_empty());
    }

    #[test]
    fn test_get_current_directory() {
        let cwd = get_current_directory();
        assert!(cwd.is_ok());
        let cwd_str = cwd.expect("cwd should be Ok after is_ok check");
        assert!(!cwd_str.is_empty());
        // Should start with / or ~
        assert!(cwd_str.starts_with('/') || cwd_str.starts_with('~'));
    }

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(hostname.is_ok());
        let hname = hostname.expect("hostname should be Ok after is_ok check");
        assert!(!hname.is_empty());
    }

    #[test]
    fn test_get_git_branch() {
        let branch = get_git_branch();
        // Since this is run in a git repo, should be Some
        assert!(branch.is_some());
        let branch_name = branch.expect("branch should be Some after is_some check");
        assert!(!branch_name.is_empty());
    }

    #[test]
    fn test_get_git_repo_name() {
        let repo_name = get_git_repo_name();
        // Since this is run in a git repo, should be Some
        assert!(repo_name.is_some());
        let name = repo_name.expect("repo_name should be Some after is_some check");
        assert_eq!(name, "pulse");
        assert!(!name.is_empty());
    }

    #[test]
    fn test_get_git_info() {
        let lazy_info = get_git_info();
        assert!(lazy_info.get().is_some());
        let info = lazy_info
            .get()
            .expect("lazy_info should be Some after is_some check");
        assert_eq!(info.repo_name, "pulse");
        assert!(!info.branch.is_empty());
        assert!(info.work_dir.is_absolute());
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
    #[serial]
    fn test_get_exit_code_default() {
        // Ensure no env vars are set
        unsafe {
            std::env::remove_var("PIPESTATUS");
            std::env::remove_var("LAST_EXIT_CODE");
        }
        assert_eq!(get_exit_code(), "0");
    }

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
        let _ = is_root_user();
    }

    #[test]
    fn test_generate_prompt_root_symbol() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.expect("prompt should be Ok after is_ok check");
        // Should contain either $ or # depending on user
        assert!(p.contains("$") || p.contains("#"));
    }

    #[test]
    fn test_generate_prompt() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.expect("prompt should be Ok after is_ok check");
        assert!(p.contains("$ "));
        assert!(p.lines().count() == 2); // DualLine mode
    }

    #[test]
    fn test_generate_prompt_inline_git() {
        let config = crate::config::Config {
            mode: Some("Inline".to_string()),
            ..Default::default()
        };
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.expect("prompt should be Ok after is_ok check");
        assert!(p.contains("$ "));
        assert!(p.lines().count() == 1); // Inline mode
    }

    #[test]
    fn test_generate_prompt_dualline_git_format() {
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.expect("prompt should be Ok after is_ok check");
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
        assert!(width.expect("terminal width should be Some") > 0);
    }

    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next(); // skip '['
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphabetic() {
                        chars.next(); // skip the letter (e.g., 'm')
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
