//! Prompt generation logic for Pulse.
//!
//! Generates shell prompts with user info, host, directory, and Git status.
//! Supports different modes and customizable colors.

use std::cell::OnceCell;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use crate::clrs::Clrs;
use crate::config::Config;
use crossterm::terminal::size;
use owo_colors::OwoColorize;

const DEFAULT_TERM_WIDTH: usize = 120;
const TRUNCATION_THRESHOLD: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    Bash,
    Zsh,
}

fn detect_shell() -> ShellKind {
    if std::env::var_os("ZSH_VERSION").is_some() {
        ShellKind::Zsh
    } else {
        ShellKind::Bash
    }
}

/// Wrap ANSI escape sequences with readline invisible-character markers so that
/// the shell can correctly calculate the visual width of the prompt. Without
/// these markers, terminal resize in multi-line prompts causes display corruption.
fn wrap_ansi_for_readline(prompt: &str, shell: ShellKind) -> String {
    let (start, end) = match shell {
        ShellKind::Bash => ("\x01", "\x02"),
        ShellKind::Zsh => ("%{", "%}"),
    };

    let mut result = String::with_capacity(prompt.len() * 2);
    let mut chars = prompt.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            result.push_str(start);
            result.push(c);
            result.push(chars.next().unwrap()); // '['
            while let Some(&next) = chars.peek() {
                result.push(chars.next().unwrap());
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            result.push_str(end);
        } else {
            result.push(c);
        }
    }

    result
}

pub fn get_terminal_width() -> Option<u16> {
    size().ok().map(|(w, _)| w)
}

/// Discover a git repository starting from the given path.
///
/// This allows tests and callers to request discovery from an explicit
/// directory, while keeping a convenience wrapper for the common
/// case of discovering from the current working directory.
fn discover_git_repo_in<P: AsRef<std::path::Path>>(path: P) -> Option<gix::Repository> {
    gix::discover(path.as_ref()).ok()
}

/// Convenience wrapper that discovers a repository from the current
/// working directory (keeps the previous public behaviour).
fn discover_git_repo() -> Option<gix::Repository> {
    discover_git_repo_in(".")
}

#[derive(Debug, Clone, Copy)]
pub struct PromptColors {
    pub user_color: owo_colors::DynColors,
    pub host_color: owo_colors::DynColors,
    pub git_color: owo_colors::DynColors,
    pub white: owo_colors::DynColors,
    pub dir_color: owo_colors::DynColors,
}

impl PromptColors {
    pub fn from_config(config: &Config) -> Self {
        Self {
            user_color: config.get_color("username").to_dyn(),
            host_color: config.get_color("hostname").to_dyn(),
            git_color: config.get_color("git_branch").to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: config.get_color("current_directory").to_dyn(),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptLayout {
    Inline,
    DualLine,
}

impl PromptLayout {
    fn from_config(mode: Option<&str>) -> Self {
        match mode {
            Some("Inline") => Self::Inline,
            _ => Self::DualLine,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            PromptLayout::Inline => "Inline",
            PromptLayout::DualLine => "DualLine",
        }
    }
}

#[derive(Debug, Clone)]
struct PromptBuilderData {
    mode: PromptLayout,
    colors: PromptColors,
    user: Option<String>,
    host: Option<String>,
    dir: Option<String>,
    current_dir: Option<PathBuf>,
    git_info: Option<GitInfo>,
    terminal_width: u16,
    exit_code: String,
    is_root: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NeedsUser;

#[derive(Debug, Clone, Copy)]
pub struct NeedsHost;

#[derive(Debug, Clone, Copy)]
pub struct NeedsDir;

#[derive(Debug, Clone, Copy)]
pub struct Ready;

#[derive(Debug, Clone)]
pub struct PromptBuilder<State> {
    data: PromptBuilderData,
    _state: PhantomData<State>,
}

impl PromptBuilder<NeedsUser> {
    pub fn from_config(config: &Config) -> Self {
        Self {
            data: PromptBuilderData {
                mode: PromptLayout::from_config(config.mode.as_deref()),
                colors: PromptColors::from_config(config),
                user: None,
                host: None,
                dir: None,
                current_dir: None,
                git_info: None,
                terminal_width: DEFAULT_TERM_WIDTH as u16,
                exit_code: "0".to_string(),
                is_root: false,
            },
            _state: PhantomData,
        }
    }

    pub fn user(mut self, user: impl Into<String>) -> PromptBuilder<NeedsHost> {
        self.data.user = Some(user.into());
        PromptBuilder {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl PromptBuilder<NeedsHost> {
    pub fn host(mut self, host: impl Into<String>) -> PromptBuilder<NeedsDir> {
        self.data.host = Some(host.into());
        PromptBuilder {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl PromptBuilder<NeedsDir> {
    pub fn dir(mut self, dir: impl Into<String>) -> PromptBuilder<Ready> {
        self.data.dir = Some(dir.into());
        PromptBuilder {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl PromptBuilder<Ready> {
    pub fn render(self) -> Result<String> {
        let user = self
            .data
            .user
            .ok_or_else(|| anyhow!("PromptBuilder missing user"))?;
        let host = self
            .data
            .host
            .ok_or_else(|| anyhow!("PromptBuilder missing host"))?;
        let dir = self
            .data
            .dir
            .ok_or_else(|| anyhow!("PromptBuilder missing dir"))?;

        let first_line = if let Some(info) = self.data.git_info {
            let nav_parts_owned = if let Some(current_dir) = &self.data.current_dir {
                let relative = current_dir.strip_prefix(&info.work_dir).unwrap_or(current_dir);
                let relative_str = relative.to_string_lossy();
                relative_str
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let nav_parts = nav_parts_owned.iter().map(String::as_str).collect::<Vec<_>>();
            let email = info.user_email.as_deref();
            let display_mode = select_display_mode(
                self.data.terminal_width,
                email,
                &info.repo_name,
                &info.branch,
                &nav_parts,
                &self.data.colors,
            );
            format_git_prompt_line(
                display_mode,
                email,
                &info.repo_name,
                &info.branch,
                &nav_parts,
                &self.data.colors,
            )
        } else {
            build_non_git_path_string(
                &dir,
                &user,
                &host,
                &self.data.colors,
                self.data.mode.as_str(),
            )
        };

        let prompt_symbol = if self.data.is_root { "#" } else { "$" };
        let prompt = match self.data.mode {
            PromptLayout::Inline => format!("{} {} ", first_line, prompt_symbol),
            PromptLayout::DualLine => format!(
                "{}\n└─ {} {} ",
                first_line, self.data.exit_code, prompt_symbol
            ),
        };
        Ok(prompt)
    }
}

impl<State> PromptBuilder<State> {
    pub fn terminal_width(mut self, terminal_width: u16) -> Self {
        self.data.terminal_width = terminal_width;
        self
    }

    pub fn exit_code(mut self, exit_code: impl Into<String>) -> Self {
        self.data.exit_code = exit_code.into();
        self
    }

    pub fn root(mut self, is_root: bool) -> Self {
        self.data.is_root = is_root;
        self
    }

    pub fn current_dir_path(mut self, current_dir: PathBuf) -> Self {
        self.data.current_dir = Some(current_dir);
        self
    }

    pub fn git_info(mut self, git_info: Option<GitInfo>) -> Self {
        self.data.git_info = git_info;
        self
    }

}

pub struct LazyGitInfo {
    repo: Option<gix::Repository>,
    cached: OnceCell<Option<GitInfo>>,
}

impl LazyGitInfo {
    pub fn new(repo: Option<gix::Repository>) -> Self {
        Self {
            repo,
            cached: OnceCell::new(),
        }
    }

    pub fn get(&self) -> Option<&GitInfo> {
        self.cached
            .get_or_init(|| {
                self.repo
                    .as_ref()
                    .and_then(build_git_info)
            })
            .as_ref()
    }
}

impl Default for LazyGitInfo {
    fn default() -> Self {
        Self::new(discover_git_repo())
    }
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
    let user = get_prompt_user()?;
    let host = get_hostname()?;
    let dir = get_current_directory()?;
    let current_dir = std::env::current_dir()?;
    let repo = discover_git_repo();
    let git_info = LazyGitInfo::new(repo);
    let git_info = git_info.get().cloned();
    let exit_code = get_exit_code();
    let terminal_width = get_terminal_width().unwrap_or(DEFAULT_TERM_WIDTH as u16);

    let prompt = PromptBuilder::from_config(config)
        .terminal_width(terminal_width)
        .current_dir_path(current_dir)
        .exit_code(exit_code)
        .root(is_root_user())
        .git_info(git_info)
        .user(user)
        .host(host)
        .dir(dir)
        .render()?;

    Ok(wrap_ansi_for_readline(&prompt, detect_shell()))
}

/// Get the system's hostname
pub fn get_hostname() -> Result<String> {
    hostname::get()
        .map(|s| s.to_string_lossy().to_string())
        .map_err(|e| anyhow::anyhow!("Unable to get hostname: {}", e))
}


fn get_git_branch_from_repo(repo: &gix::Repository) -> Option<String> {
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


fn build_git_info(repo: &gix::Repository) -> Option<GitInfo> {
    let work_dir = repo.work_dir()?;
    let work_dir = std::fs::canonicalize(work_dir).ok()?;
    let repo_name = work_dir.file_name()?.to_str()?.to_string();
    let branch = get_git_branch_from_repo(repo)
        .unwrap_or_else(|| "unknown".to_string());

    let config = repo.config_snapshot();
    let user_email = config.string("user.email").map(|s| s.to_string());

    Some(GitInfo {
        repo_name,
        branch,
        user_email,
        work_dir,
    })
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
pub fn truncate_non_git_path(root: &str, parts: &[&str]) -> String {
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
    _mode: &str,
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
    let path_display = truncate_non_git_path(root, &nav_parts);

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
    use tempfile::TempDir;

    struct DirGuard {
        original: PathBuf,
    }

    impl DirGuard {
        fn new(target: &std::path::Path) -> Self {
            let original = std::env::current_dir().expect("current dir should be available");
            std::env::set_current_dir(target).expect("set current dir to temp repo");
            Self { original }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    fn init_temp_git_repo() -> TempDir {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let _repo = gix::init(temp_dir.path()).expect("init temp git repo");
        temp_dir
    }

    fn repo_name_from_path(path: &std::path::Path) -> String {
        path.file_name()
            .and_then(|name| name.to_str())
            .expect("temp repo dir name")
            .to_string()
    }

    #[test]
    #[serial]
    fn test_is_in_git_repo() {
        let temp_dir = init_temp_git_repo();
        let repo = discover_git_repo_in(temp_dir.path());
        assert!(repo.is_some());
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
    #[serial]
    fn test_get_git_branch() {
        let temp_dir = init_temp_git_repo();
        let repo = discover_git_repo_in(temp_dir.path()).expect("repo");
        let branch = get_git_branch_from_repo(&repo);
        assert!(branch.is_some());
        let branch_name = branch.expect("branch should be Some after is_some check");
        assert!(!branch_name.is_empty());
    }

    #[test]
    #[serial]
    fn test_get_git_info() {
        let temp_dir = init_temp_git_repo();
        let repo = discover_git_repo_in(temp_dir.path()).expect("repo");
        let expected = repo_name_from_path(temp_dir.path());
        let info = build_git_info(&repo).expect("build git info");
        assert_eq!(info.repo_name, expected);
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
        assert_eq!(truncate_non_git_path("~", &["a", "b"]), "~ a › b");
    }

    #[test]
    fn test_truncate_non_git_path_dualline_empty() {
        assert_eq!(truncate_non_git_path("/", &[]), "/");
    }

    #[test]
    fn test_truncate_non_git_path_tilde_empty() {
        assert_eq!(truncate_non_git_path("~", &[]), "~");
    }

    #[test]
    fn test_truncate_non_git_path_dualline_three_parts() {
        assert_eq!(
            truncate_non_git_path("~", &["home", "user", "docs"]),
            "~ home › user › docs"
        );
    }

    #[test]
    fn test_truncate_non_git_path_dualline_four_parts() {
        assert_eq!(
            truncate_non_git_path("/", &["usr", "local", "bin", "pulse"]),
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
    #[serial]
    fn test_generate_prompt_inline_git() {
        let temp_dir = init_temp_git_repo();
        let _guard = DirGuard::new(temp_dir.path());
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
    #[serial]
    fn test_generate_prompt_dualline_git_format() {
        let temp_dir = init_temp_git_repo();
        let _guard = DirGuard::new(temp_dir.path());
        let expected = repo_name_from_path(temp_dir.path());
        let config = crate::config::Config::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.is_ok());
        let p = prompt.expect("prompt should be Ok after is_ok check");
        // Should contain repo name and branch in Git format
        assert!(p.contains(&expected));
        assert!(p.contains("[")); // start of Git info
        assert!(p.contains(" : ")); // separator
        assert!(p.contains("]")); // end of Git info
        // Should have navigation path
        assert!(p.lines().count() == 2);
    }

    #[test]
    fn test_get_terminal_width() {
        let width = get_terminal_width();
        let Some(width) = width else {
            return;
        };
        assert!(width > 0);
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

    fn make_test_colors() -> PromptColors {
        use crate::clrs::Clrs;
        PromptColors {
            user_color: Clrs::Aqua.to_dyn(),
            host_color: Clrs::Yellow.to_dyn(),
            git_color: Clrs::Green.to_dyn(),
            white: Clrs::White.to_dyn(),
            dir_color: Clrs::Blue.to_dyn(),
        }
    }

    fn make_color_override_config() -> Config {
        let mut config = Config::default();
        for segment in &mut config.segments {
            match segment.name.as_str() {
                "username" => segment.color = Some("Red".to_string()),
                "hostname" => segment.color = Some("Yellow".to_string()),
                "current_directory" => segment.color = Some("Blue".to_string()),
                "git_branch" => segment.color = Some("Green".to_string()),
                _ => {}
            }
        }
        config.segment_colors.clear();
        config.segment_colors.insert("username".to_string(), Clrs::Red);
        config.segment_colors.insert("hostname".to_string(), Clrs::Yellow);
        config.segment_colors.insert("current_directory".to_string(), Clrs::Blue);
        config.segment_colors.insert("git_branch".to_string(), Clrs::Green);
        config
    }

    #[test]
    fn test_prompt_colors_from_config() {
        let config = make_color_override_config();
        let colors = PromptColors::from_config(&config);

        let user = format!("{}", "user".color(colors.user_color));
        let host = format!("{}", "host".color(colors.host_color));
        let git = format!("{}", "git".color(colors.git_color));
        let dir = format!("{}", "dir".color(colors.dir_color));
        let white = format!("{}", "white".color(colors.white));

        assert_eq!(user, format!("{}", "user".color(Clrs::Red.to_dyn())));
        assert_eq!(host, format!("{}", "host".color(Clrs::Yellow.to_dyn())));
        assert_eq!(git, format!("{}", "git".color(Clrs::Green.to_dyn())));
        assert_eq!(dir, format!("{}", "dir".color(Clrs::Blue.to_dyn())));
        assert_eq!(white, format!("{}", "white".color(Clrs::White.to_dyn())));
    }

    #[test]
    fn test_select_display_mode_nano() {
        let result = select_display_mode(
            10,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src", "lib"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Nano);
    }

    #[test]
    fn test_select_display_mode_micro() {
        let result = select_display_mode(
            33,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Micro);
    }

    #[test]
    fn test_select_display_mode_mini() {
        let result = select_display_mode(
            36,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Mini);
    }

    #[test]
    fn test_select_display_mode_full() {
        let result = select_display_mode(
            200,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src", "main", "rust"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Full);
    }

    #[test]
    fn test_select_display_mode_auto_wide_terminal() {
        let result = select_display_mode(
            200,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src", "main"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Full);
    }

    #[test]
    fn test_select_display_mode_auto_medium_terminal() {
        let result = select_display_mode(
            36,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Mini);
    }

    #[test]
    fn test_select_display_mode_auto_narrow_terminal() {
        let result = select_display_mode(
            25,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Nano);
    }

    #[test]
    fn test_select_display_mode_auto_zero_width() {
        let result = select_display_mode(
            0,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Nano);
    }

    #[test]
    fn test_select_display_mode_auto_very_small_width() {
        let result = select_display_mode(
            5,
            Some("user@example.com"),
            "myrepo",
            "main",
            &["src"],
            &make_test_colors(),
        );
        assert_eq!(result, GitDisplayMode::Nano);
    }

    #[test]
    fn test_prompt_builder_inline_non_git() {
        let config = Config {
            mode: Some("Inline".to_string()),
            ..Default::default()
        };
        let prompt = PromptBuilder::from_config(&config)
            .terminal_width(120)
            .exit_code("0")
            .root(false)
            .user("alice")
            .host("devbox")
            .dir("~/work/pulse")
            .render();

        let prompt = match prompt {
            Ok(value) => value,
            Err(err) => panic!("prompt render failed: {err}"),
        };

        let clean = strip_ansi(&prompt);
        assert!(clean.lines().count() == 1);
        assert_eq!(clean, "alice@devbox:~ work › pulse $ ");
    }

    #[test]
    fn test_prompt_builder_dualline_non_git() {
        let config = Config::default();
        let prompt = PromptBuilder::from_config(&config)
            .terminal_width(120)
            .exit_code("7")
            .root(false)
            .user("bob")
            .host("laptop")
            .dir("/usr/local/bin")
            .render();

        let prompt = match prompt {
            Ok(value) => value,
            Err(err) => panic!("prompt render failed: {err}"),
        };

        let clean = strip_ansi(&prompt);
        let mut lines = clean.lines();
        let first = lines.next().unwrap_or("");
        let second = lines.next().unwrap_or("");
        assert!(lines.next().is_none());
        assert_eq!(first, "bob@laptop:/ usr › local › bin");
        assert_eq!(second, "└─ 7 $ ");
    }

    #[test]
    fn test_prompt_builder_inline_git() {
        let config = Config {
            mode: Some("Inline".to_string()),
            ..Default::default()
        };
        let git_info = GitInfo {
            repo_name: "pulse".to_string(),
            branch: "main".to_string(),
            user_email: Some("dev@example.com".to_string()),
            work_dir: PathBuf::from("/repo"),
        };
        let prompt = PromptBuilder::from_config(&config)
            .terminal_width(200)
            .exit_code("0")
            .root(false)
            .git_info(Some(git_info))
            .current_dir_path(PathBuf::from("/repo/src/lib"))
            .user("unused")
            .host("unused")
            .dir("/repo/src/lib")
            .render();

        let prompt = match prompt {
            Ok(value) => value,
            Err(err) => panic!("prompt render failed: {err}"),
        };

        let clean = strip_ansi(&prompt);
        assert!(clean.lines().count() == 1);
        assert_eq!(clean, "dev@example.com: [pulse : main] src › lib $ ");
    }

    #[test]
    fn test_prompt_builder_dualline_git() {
        let config = Config::default();
        let git_info = GitInfo {
            repo_name: "pulse".to_string(),
            branch: "main".to_string(),
            user_email: Some("dev@example.com".to_string()),
            work_dir: PathBuf::from("/repo"),
        };
        let prompt = PromptBuilder::from_config(&config)
            .terminal_width(200)
            .exit_code("9")
            .root(false)
            .git_info(Some(git_info))
            .current_dir_path(PathBuf::from("/repo/src/bin"))
            .user("unused")
            .host("unused")
            .dir("/repo/src/bin")
            .render();

        let prompt = match prompt {
            Ok(value) => value,
            Err(err) => panic!("prompt render failed: {err}"),
        };

        let clean = strip_ansi(&prompt);
        let mut lines = clean.lines();
        let first = lines.next().unwrap_or("");
        let second = lines.next().unwrap_or("");
        assert!(lines.next().is_none());
        assert_eq!(first, "dev@example.com: [pulse : main] src › bin");
        assert_eq!(second, "└─ 9 $ ");
    }

    #[test]
    fn wrap_ansi_for_readline_bash_wraps_escape_sequences() {
        let input = "\x1b[38;2;0;116;217mhello\x1b[0m world";
        let result = wrap_ansi_for_readline(input, ShellKind::Bash);
        assert_eq!(result, "\x01\x1b[38;2;0;116;217m\x02hello\x01\x1b[0m\x02 world");
    }

    #[test]
    fn wrap_ansi_for_readline_zsh_wraps_escape_sequences() {
        let input = "\x1b[31mred\x1b[0m";
        let result = wrap_ansi_for_readline(input, ShellKind::Zsh);
        assert_eq!(result, "%{\x1b[31m%}red%{\x1b[0m%}");
    }

    #[test]
    fn wrap_ansi_for_readline_no_escape_sequences() {
        let input = "plain text";
        let result = wrap_ansi_for_readline(input, ShellKind::Bash);
        assert_eq!(result, "plain text");
    }

    #[test]
    fn wrap_ansi_for_readline_preserves_newline() {
        let input = "\x1b[32mline1\x1b[0m\n└─ \x1b[33m0\x1b[0m $ ";
        let result = wrap_ansi_for_readline(input, ShellKind::Bash);
        assert_eq!(
            result,
            "\x01\x1b[32m\x02line1\x01\x1b[0m\x02\n└─ \x01\x1b[33m\x020\x01\x1b[0m\x02 $ "
        );
    }
}
