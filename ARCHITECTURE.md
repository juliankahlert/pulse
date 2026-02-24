# Architecture

Pulse is a fast, configurable PS1 prompt engine written in Rust. The binary is
invoked by your shell to render the prompt on demand.

## High-Level Flow

1. Parse CLI arguments.
2. Load configuration from the default locations (global then user).
3. Assemble prompt segments based on configuration and runtime context.
4. Render the prompt as text with ANSI color sequences.

## Modules

- `src/main.rs`: entry point, logging, and top-level error handling.
- `src/args.rs`: CLI parsing and flags.
- `src/config.rs`: configuration loading, validation, and merging.
- `src/clrs.rs`: color palette support (clrs.cc-inspired).
- `src/prompt.rs`: prompt generation and segment formatting.

## Configuration Sources

Pulse reads configuration in order of precedence:

1. Built-in defaults.
2. System config at XDG config directories (`/etc/xdg/pulse/config.yaml`).
3. User config (`$XDG_CONFIG_HOME/pulse/config.yaml`).

If a segment exists in multiple sources, the last-loaded config overrides
earlier ones by segment name.

## Segment Rendering

Segments are identified by name and mapped to colors. When no custom color is
defined, Pulse uses terminal ANSI colors that adapt to your terminal palette.
On truecolor terminals, explicit RGB values are used.
