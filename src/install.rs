//! Installation logic for Pulse shell integration.

use anyhow::{Context, Result};
use std::path::PathBuf;

const INSTALL_START_MARKER: &str = "# >>> Pulse >>>";
const INSTALL_END_MARKER: &str = "# <<< Pulse <<<";

const BASH_INSTALL_COMMENT: &str = "# Pulse - PS1 prompt engine";
const BASH_EXPORT_PS1: &str = r#"export PS1='$(pulse)'"#;
const BASH_PROMPT_COMMAND: &str = r#"export PROMPT_COMMAND='export LAST_EXIT_CODE=$?'"#;

const ZSH_INSTALL_COMMENT: &str = "# Pulse - PS1 prompt engine";
const ZSH_EXPORT_PS1: &str = r#"export PS1='$(pulse)'"#;
const ZSH_PROMPT_COMMAND: &str = r#"export PROMPT_COMMAND='export LAST_EXIT_CODE=$?'"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    Bash,
    Zsh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallBlockStatus {
    None,
    Incomplete,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallAction {
    SkipIncomplete,
    Append,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UninstallAction {
    SkipIncomplete,
    NotInstalled,
    Remove,
}

struct InstallBlockFlowStart {
    content: String,
}

impl InstallBlockFlowStart {
    fn new(content: String) -> Self {
        Self { content }
    }

    fn analyze(self) -> InstallBlockFlowAnalyzed {
        let status = install_block_status(&self.content);
        InstallBlockFlowAnalyzed {
            content: self.content,
            status,
        }
    }
}

struct InstallBlockFlowAnalyzed {
    content: String,
    status: InstallBlockStatus,
}

impl InstallBlockFlowAnalyzed {
    fn plan_install(self, shell: ShellKind, rc_path: PathBuf, dry_run: bool) -> InstallPlan {
        let action = match self.status {
            InstallBlockStatus::None => InstallAction::Append,
            InstallBlockStatus::Incomplete => InstallAction::SkipIncomplete,
            InstallBlockStatus::Complete => InstallAction::Replace,
        };

        InstallPlan {
            content: self.content,
            shell,
            rc_path,
            dry_run,
            action,
        }
    }

    fn plan_uninstall(self, rc_path: PathBuf, dry_run: bool) -> UninstallPlan {
        let action = match self.status {
            InstallBlockStatus::None => UninstallAction::NotInstalled,
            InstallBlockStatus::Incomplete => UninstallAction::SkipIncomplete,
            InstallBlockStatus::Complete => UninstallAction::Remove,
        };

        UninstallPlan {
            content: self.content,
            rc_path,
            dry_run,
            action,
        }
    }
}

struct InstallPlan {
    content: String,
    shell: ShellKind,
    rc_path: PathBuf,
    dry_run: bool,
    action: InstallAction,
}

impl InstallPlan {
    fn apply(self) -> Result<()> {
        match self.action {
            InstallAction::SkipIncomplete => {
                println!(
                    "Pulse markers detected in {} but block is incomplete; not modifying file.",
                    self.rc_path.display()
                );
            }
            InstallAction::Append => {
                let install_block = install_block_for(self.shell);
                let updated = append_block_to_content(&self.content, &install_block);
                write_content(&self.rc_path, &updated, self.dry_run)?;
                if self.dry_run {
                    println!("Dry run: would install Pulse to {}", self.rc_path.display());
                } else {
                    println!("Pulse has been installed to {}", self.rc_path.display());
                    println!(
                        "Please restart your shell or run 'source {}' to apply changes.",
                        self.rc_path.display()
                    );
                }
            }
            InstallAction::Replace => {
                println!("Pulse is already installed in {}", self.rc_path.display());
                if self.dry_run {
                    println!(
                        "Dry run: would replace existing Pulse install block in {}",
                        self.rc_path.display()
                    );
                    return Ok(());
                }

                println!("Removing existing installation for upgrade...");
                let (stripped, removed) = remove_install_block(&self.content);
                if removed {
                    println!("Existing installation removed successfully");
                }

                let install_block = install_block_for(self.shell);
                let updated = append_block_to_content(&stripped, &install_block);
                write_content(&self.rc_path, &updated, self.dry_run)?;
                println!("Pulse has been installed to {}", self.rc_path.display());
                println!(
                    "Please restart your shell or run 'source {}' to apply changes.",
                    self.rc_path.display()
                );
            }
        }

        Ok(())
    }
}

struct UninstallPlan {
    content: String,
    rc_path: PathBuf,
    dry_run: bool,
    action: UninstallAction,
}

impl UninstallPlan {
    fn apply(self) -> Result<()> {
        match self.action {
            UninstallAction::NotInstalled => {
                println!("Pulse is not installed in {}", self.rc_path.display());
            }
            UninstallAction::SkipIncomplete => {
                println!(
                    "Pulse markers detected in {} but block is incomplete; not modifying file.",
                    self.rc_path.display()
                );
            }
            UninstallAction::Remove => {
                let (updated, removed) = remove_install_block(&self.content);
                if removed {
                    if self.dry_run {
                        println!(
                            "Dry run: would remove Pulse installation from {}",
                            self.rc_path.display()
                        );
                    } else {
                        write_content(&self.rc_path, &updated, self.dry_run)?;
                        println!("Pulse has been uninstalled from {}", self.rc_path.display());
                    }
                }
            }
        }

        Ok(())
    }
}

fn detect_shell() -> ShellKind {
    match std::env::var("SHELL") {
        Ok(shell) if shell.ends_with("zsh") => ShellKind::Zsh,
        Ok(shell) if shell.ends_with("bash") => ShellKind::Bash,
        _ => ShellKind::Bash,
    }
}

fn get_shell_rc(shell: ShellKind) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;

    let rc_path = match shell {
        ShellKind::Zsh => home.join(".zshrc"),
        ShellKind::Bash => home.join(".bashrc"),
    };

    Ok(rc_path)
}

fn install_block_status(content: &str) -> InstallBlockStatus {
    let mut found_start = false;
    let mut found_end_without_start = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == INSTALL_START_MARKER {
            found_start = true;
            continue;
        }
        if trimmed == INSTALL_END_MARKER {
            if found_start {
                return InstallBlockStatus::Complete;
            }
            found_end_without_start = true;
        }
    }

    if found_start || found_end_without_start {
        InstallBlockStatus::Incomplete
    } else {
        InstallBlockStatus::None
    }
}

fn append_block_to_content(content: &str, block: &str) -> String {
    let mut updated = content.to_string();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(block);
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated
}

fn write_content(path: &PathBuf, content: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    std::fs::write(path, content).with_context(|| format!("Failed to write to {}", path.display()))
}

fn remove_install_block(content: &str) -> (String, bool) {
    let mut filtered_lines: Vec<String> = Vec::new();
    let mut removed = false;
    let mut skip_pulse_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == INSTALL_START_MARKER {
            skip_pulse_block = true;
            removed = true;
            continue;
        }

        if skip_pulse_block {
            if trimmed == INSTALL_END_MARKER {
                skip_pulse_block = false;
            }
            removed = true;
            continue;
        }

        filtered_lines.push(line.to_string());
    }

    let mut joined = filtered_lines.join("\n");
    if content.ends_with('\n') {
        joined.push('\n');
    }

    (joined, removed)
}

/// Install Pulse to the shell's RC file.
///
/// This function adds Pulse initialization code to the user's shell
/// configuration file (.bashrc or .zshrc based on the current shell).
///
/// # Preconditions
/// - If the `SHELL` environment variable is unset or unsupported, defaults to bash.
/// - The home directory must be accessible via the `dirs` crate.
///
/// # Postconditions
/// - If not already installed, adds the Pulse initialization lines to the RC file.
/// - If already installed, removes the old installation before adding the new one.
/// - Prints status messages to stdout indicating the installation result.
///
/// # Error Cases
/// Returns an error if:
/// - The home directory cannot be determined (via `get_shell_rc()`).
/// - The RC file cannot be read.
/// - The RC file cannot be written to (via `write_content()`).
///
/// # Example
/// ```ignore
/// if let Err(e) = install() {
///     eprintln!("Installation failed: {}", e);
/// }
/// ```
fn install_block_for(shell: ShellKind) -> String {
    let (comment, ps1_line, prompt_command_line) = match shell {
        ShellKind::Zsh => (ZSH_INSTALL_COMMENT, ZSH_EXPORT_PS1, ZSH_PROMPT_COMMAND),
        ShellKind::Bash => (BASH_INSTALL_COMMENT, BASH_EXPORT_PS1, BASH_PROMPT_COMMAND),
    };

    format!(
        "{}\n{}\n{}\n{}\n{}",
        INSTALL_START_MARKER, comment, ps1_line, prompt_command_line, INSTALL_END_MARKER
    )
}

pub fn install(dry_run: bool) -> Result<()> {
    let shell = detect_shell();
    let rc_path = get_shell_rc(shell)?;
    if shell == ShellKind::Bash {
        match std::env::var("SHELL") {
            Ok(shell) if shell.ends_with("bash") => {}
            _ => {
                println!(
                    "SHELL is not set or unsupported; defaulting to bash rc: {}",
                    rc_path.display()
                );
            }
        }
    }

    let existing = match std::fs::read_to_string(&rc_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };

    InstallBlockFlowStart::new(existing)
        .analyze()
        .plan_install(shell, rc_path, dry_run)
        .apply()
}

pub fn uninstall(dry_run: bool) -> Result<()> {
    let shell = detect_shell();
    let rc_path = get_shell_rc(shell)?;
    if shell == ShellKind::Bash {
        match std::env::var("SHELL") {
            Ok(shell) if shell.ends_with("bash") => {}
            _ => {
                println!(
                    "SHELL is not set or unsupported; defaulting to bash rc: {}",
                    rc_path.display()
                );
            }
        }
    }

    let content = match std::fs::read_to_string(&rc_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "No shell rc found at {}; nothing to uninstall.",
                rc_path.display()
            );
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    InstallBlockFlowStart::new(content)
        .analyze()
        .plan_uninstall(rc_path, dry_run)
        .apply()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_install_block_status_transitions() {
        assert_eq!(
            install_block_status("some content"),
            InstallBlockStatus::None
        );
        assert_eq!(
            install_block_status("# >>> Pulse >>>\npartial"),
            InstallBlockStatus::Incomplete
        );
        assert_eq!(
            install_block_status("# <<< Pulse <<<"),
            InstallBlockStatus::Incomplete
        );
        assert_eq!(
            install_block_status("# >>> Pulse >>>\n# <<< Pulse <<<"),
            InstallBlockStatus::Complete
        );
    }

    #[test]
    fn test_install_plan_skips_incomplete_markers() {
        let content = "# >>> Pulse >>>\npartial".to_string();
        let plan = InstallBlockFlowStart::new(content).analyze().plan_install(
            ShellKind::Bash,
            PathBuf::from("/tmp/rc"),
            true,
        );

        assert_eq!(plan.action, InstallAction::SkipIncomplete);
    }

    #[test]
    fn test_uninstall_plan_skips_incomplete_markers() {
        let content = "# <<< Pulse <<<".to_string();
        let plan = InstallBlockFlowStart::new(content)
            .analyze()
            .plan_uninstall(PathBuf::from("/tmp/rc"), true);

        assert_eq!(plan.action, UninstallAction::SkipIncomplete);
    }

    #[test]
    fn test_install_plan_dry_run_does_not_write() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_path_buf();
        std::fs::write(&path, "original")?;

        let plan = InstallBlockFlowStart::new("original".to_string())
            .analyze()
            .plan_install(ShellKind::Bash, path.clone(), true);

        plan.apply()?;

        let remaining = std::fs::read_to_string(&path)?;
        assert_eq!(remaining, "original");
        Ok(())
    }

    #[test]
    fn test_uninstall_plan_dry_run_does_not_write() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_path_buf();
        let content = "before\n# >>> Pulse >>>\nblock\n# <<< Pulse <<<\nafter";
        std::fs::write(&path, content)?;

        let plan = InstallBlockFlowStart::new(content.to_string())
            .analyze()
            .plan_uninstall(path.clone(), true);

        plan.apply()?;

        let remaining = std::fs::read_to_string(&path)?;
        assert_eq!(remaining, content);
        Ok(())
    }

    #[test]
    fn test_install_plan_replace_for_complete_block() {
        let content = "# >>> Pulse >>>\nblock\n# <<< Pulse <<<".to_string();
        let plan = InstallBlockFlowStart::new(content).analyze().plan_install(
            ShellKind::Bash,
            PathBuf::from("/tmp/rc"),
            false,
        );

        assert_eq!(plan.action, InstallAction::Replace);
    }

    #[test]
    fn test_remove_install_block_without_trailing_blank_line() {
        let content = "before\n# >>> Pulse >>>\n# Pulse - PS1 prompt engine\nexport PS1='$(pulse)'\nexport PROMPT_COMMAND='export LAST_EXIT_CODE=$?'\n# <<< Pulse <<<\nafter";
        let (remaining, removed) = remove_install_block(content);

        assert!(removed);
        assert_eq!(remaining, "before\nafter");
    }
}
