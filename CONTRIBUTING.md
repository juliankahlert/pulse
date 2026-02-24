# Contributing to Pulse

Thanks for taking the time to contribute to Pulse! This guide explains how to get started.

## Getting Started

1. Fork the repository and create your branch from `main`.
2. Install Rust (latest stable). See <https://www.rust-lang.org/tools/install>.
3. Build the project:

```bash
cargo build
```

## Development Guidelines

- Keep changes focused and minimal.
- Follow the existing style (Rust Edition 2024, no panics).
- Prefer clear, well-documented code over cleverness.
- Update documentation when behavior changes.

## Testing

Run the test suite before submitting:

```bash
cargo test
```

## Submitting Changes

1. Ensure your branch is up to date with `main`.
2. Open a pull request with a clear description of the change and why it is needed.
3. Link any relevant issues.

## Reporting Issues

If you find a bug or have a feature request, please open an issue with:

- Steps to reproduce (if applicable)
- Expected vs. actual behavior
- Environment details (OS, shell, Rust version)
