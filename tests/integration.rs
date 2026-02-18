use std::process::Command;

#[test]
fn test_pulse_binary_runs_and_outputs_prompt() {
    // Build the release binary first
    let build_status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build pulse");

    assert!(build_status.success(), "Cargo build failed");

    // Run the pulse binary
    let output = Command::new("./target/release/pulse")
        .output()
        .expect("Failed to run pulse binary");

    // Assert it ran successfully
    assert!(output.status.success(), "Pulse binary exited with error");

    // Assert it produced output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Pulse binary produced no output");

    // Check for basic prompt elements
    assert!(stdout.contains('@'), "Output should contain @ separator");
    assert!(stdout.contains(':'), "Output should contain : separator");

    // Test with LAST_EXIT_CODE set
    let output_with_exit = Command::new("./target/release/pulse")
        .env("LAST_EXIT_CODE", "42")
        .output()
        .expect("Failed to run pulse with LAST_EXIT_CODE");

    assert!(output_with_exit.status.success());
    let stdout_exit = String::from_utf8_lossy(&output_with_exit.stdout);
    assert!(!stdout_exit.is_empty());
    // Assuming dualline mode shows exit code, check if it contains the exit code
    // The exact format depends on implementation, but should be present
    assert!(
        stdout_exit.contains("42") || stdout_exit.contains("└─"),
        "Output should reflect exit code or dualline format"
    );
}
