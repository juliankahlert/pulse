# pulse
Pulse – A fast, configurable Rust PS1 prompt engine for modern shells.

## Installation

### Prerequisites
- Rust (latest stable version recommended)

### Building from Source
```bash
git clone https://github.com/juliankahlert/pulse.git
cd pulse
cargo build --release
```

The binary will be available at `target/release/pulse`.

### Usage
To use Pulse as your shell prompt, set the `PS1` environment variable in your shell configuration (e.g., `~/.bashrc`, `~/.zshrc`):

```bash
export PS1="\$(pulse)"
```

To display the exit code of the last command in the prompt, also add the following to your shell configuration:

```bash
export PROMPT_COMMAND='export LAST_EXIT_CODE=$?'
```

This will execute `pulse` every time the prompt is displayed, generating the customized prompt with the correct exit code.

## Configuration
Configure the prompt via `/etc/pulse/config.yaml` (global) and `~/.config/pulse/config.yaml` (user-specific).

### Segment Coloring
Pulse supports coloring different segments of the prompt using a predefined color palette inspired by [clrs.cc](https://clrs.cc/). You can specify colors for various prompt segments in your configuration file.

Default is coloring for dark background.

#### Available Colors
The following colors are available for use in segment coloring:

- Navy
- Blue
- Aqua
- Teal
- Olive
- Green
- Lime
- Yellow
- Orange
- Red
- Maroon
- Fuchsia
- Purple
- Black
- Gray
- Silver
- White
- Magenta

#### Example Configuration
```yaml
segments:
  - name: username
    color: Blue
  - name: hostname
    color: Green
  - name: current_directory
    color: Navy
```

## Examples

Default is dialline with git

### Inline Mode
A compact single-line prompt:
```
user@host:~ pulse $
```

**Description:** The prompt consists of segments: username (user), separator (@), hostname (host), separator (:), current directory (~), space, prompt name (pulse), space, and shell prompt ($). No UTF-8 special characters are used.

### Dualline Mode
A multi-line prompt with path navigation:
```
user@host:~ pulse › src › main
└─ 0 $
```

**Description:** Multi-line prompt with first line showing: username@user@host:~ pulse › src › main (segments: username, @, hostname, :, current dir ~, space, pulse, space, › (U+203A) separator, directory segments src › main). Second line: └─ (U+2514 U+2500) separator, exit code of the last command (0), space, $.

A multi-line prompt with deep path navigation and root starting in home dir:
```
user@host:~ … pulse › src › main
└─ 0 $
```

**Description:** Similar to above, but with ellipsis (…) (U+2026) indicating truncated path (path segments are truncated if there are more than 3) starting from home (~). First line: user@host:~ … pulse › src › main. Second line: └─ (U+2514 U+2500) separator, exit code of the last command (0), space, $.

A multi-line prompt with deep path navigation and root NOT starting in home dir:
```
user@host:/ … hwmon › hwmon2 › power
└─ 0 $
```

**Description:** Path starts from root (/), with ellipsis (…) (U+2026) for truncation (path segments are truncated if there are more than 3). First line: user@host:/ … hwmon › hwmon2 › power. Second line: └─ (U+2514 U+2500) separator, exit code of the last command (0), space, $.

### Git Support
A multi-line prompt with path navigation with a subpath of a git repository:
```
user@host: [repository-name] src › main
└─ 0 $
```

**Description:** Includes git repository name in brackets. First line: user@host: [repository-name : branch] src › main. Segments: username, @, hostname, :, space, [repository : branch], space, directory segments. Second line: └─ (U+2514 U+2500) separator, exit code of the last command (0), space, $.

