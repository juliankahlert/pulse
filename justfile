rpm:
    cargo build --release
    strip -s target/release/pulse
    mkdir -p completions
    target/release/pulse --generate-completions bash > completions/pulse.bash
    target/release/pulse --generate-completions zsh > completions/pulse.zsh
    cargo generate-rpm
    ln -sf target/generate-rpm/pulse-0.1.0-1.x86_64.rpm pulse.rpm

reinstall:
    just rpm
    sudo dnf reinstall pulse.rpm
