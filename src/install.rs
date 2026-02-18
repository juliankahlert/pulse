//! Installation logic for Pulse shell integration.

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

const BASH_INSTALL_COMMENT: &str = "# Pulse - PS1 prompt engine";
const BASH_EXPORT_PS1: &str = r#"export PS1='$(pulse)'"#;
const BASH_PROMPT_COMMAND: &str = r#"export PROMPT_COMMAND='export LAST_EXIT_CODE=$?'"#;

const ZSH_INSTALL_COMMENT: &str = "# Pulse - PS1 prompt engine";
const ZSH_EXPORT_PS1: &str = r#"export PS1='$(pulse)'"#;
const ZSH_PROMPT_COMMAND: &str = r#"export PROMPT_COMMAND='export LAST_EXIT_CODE=$?'"#;

fn get_shell_rc() -> Result<PathBuf> {
    let shell = std::env::var("SHELL").context("SHELL environment variable not set")?;

    let rc_path = if shell.ends_with("zsh") {
        dirs::home_dir()
            .map(|home| home.join(".zshrc"))
            .context("Could not determine home directory")?
    } else {
        dirs::home_dir()
            .map(|home| home.join(".bashrc"))
            .context("Could not determine home directory")?
    };

    Ok(rc_path)
}

fn shell_is_zsh() -> bool {
    std::env::var("SHELL")
        .map(|s| s.ends_with("zsh"))
        .unwrap_or(false)
}

pub fn append_to_file(path: &PathBuf, content: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    writeln!(file, "{}", content)
        .with_context(|| format!("Failed to write to {}", path.display()))?;

    Ok(())
}

pub fn is_installed(path: &PathBuf) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.contains(r#"export PS1='$(pulse)'"#))
}

pub fn remove_existing_install(path: &PathBuf) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut filtered_lines = Vec::new();
    let mut removed = false;
    let mut skip_pulse_block = false;

    for line in lines {
        if line.contains("# Pulse - PS1 prompt engine") {
            skip_pulse_block = true;
            removed = true;
            continue;
        }

        if skip_pulse_block {
            if line.is_empty() {
                skip_pulse_block = false;
            }
            continue;
        }

        filtered_lines.push(line);
    }

    if removed {
        let new_content = filtered_lines.join("\n");
        std::fs::write(path, new_content)?;
    }

    Ok(removed)
}

pub fn install() -> Result<()> {
    let rc_path = get_shell_rc()?;

    if is_installed(&rc_path)? {
        println!("Pulse is already installed in {}", rc_path.display());
        println!("Removing existing installation for upgrade...");
        if remove_existing_install(&rc_path)? {
            println!("Existing installation removed successfully");
        }
    }

    let is_zsh = shell_is_zsh();

    let (comment, ps1_line, prompt_command_line) = if is_zsh {
        (ZSH_INSTALL_COMMENT, ZSH_EXPORT_PS1, ZSH_PROMPT_COMMAND)
    } else {
        (BASH_INSTALL_COMMENT, BASH_EXPORT_PS1, BASH_PROMPT_COMMAND)
    };

    append_to_file(&rc_path, "")?;
    append_to_file(&rc_path, comment)?;
    append_to_file(&rc_path, ps1_line)?;
    append_to_file(&rc_path, prompt_command_line)?;

    println!("Pulse has been installed to {}", rc_path.display());
    println!(
        "Please restart your shell or run 'source {}' to apply changes.",
        rc_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_installed_when_file_contains_marker() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"some content\nexport PS1='$(pulse)'\nmore content")
            .expect("Failed to write to temp file");

        let path = temp_file.path().to_path_buf();
        let result = is_installed(&path).expect("is_installed should not error");

        assert_eq!(result, true);
    }

    #[test]
    fn test_is_installed_when_file_does_not_contain_marker() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"some content\nother line\nmore content")
            .expect("Failed to write to temp file");

        let path = temp_file.path().to_path_buf();
        let result = is_installed(&path).expect("is_installed should not error");

        assert_eq!(result, false);
    }

    #[test]
    fn test_is_installed_when_file_does_not_exist() {
        let path = PathBuf::from("/nonexistent/path/to/file/that/does/not/exist");
        let result = is_installed(&path);

        assert!(result.is_err());
    }

    #[test]
    fn test_append_to_file() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        append_to_file(&path, "first line").expect("append_to_file should not error");
        append_to_file(&path, "second line").expect("append_to_file should not error");

        let mut file = std::fs::File::open(&path).expect("Failed to open temp file");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read temp file");

        assert_eq!(contents, "first line\nsecond line\n");
    }

    #[test]
    fn test_remove_existing_install_removes_pulse_block() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let content = "some initial content\n# Pulse - PS1 prompt engine\nexport PS1='$(pulse)'\nexport PROMPT_COMMAND='export LAST_EXIT_CODE=$?'\n\nmore content";
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");

        let path = temp_file.path().to_path_buf();
        let removed =
            remove_existing_install(&path).expect("remove_existing_install should not error");

        assert_eq!(removed, true);

        let remaining = std::fs::read_to_string(&path).expect("Failed to read temp file");
        assert_eq!(remaining, "some initial content\nmore content");
    }

    #[test]
    fn test_remove_existing_install_when_no_pulse_block() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"some content\nother line")
            .expect("Failed to write to temp file");

        let path = temp_file.path().to_path_buf();
        let removed =
            remove_existing_install(&path).expect("remove_existing_install should not error");

        assert_eq!(removed, false);

        let remaining = std::fs::read_to_string(&path).expect("Failed to read temp file");
        assert_eq!(remaining, "some content\nother line");
    }
}
