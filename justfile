rpm:
    cargo build --release
    strip -s target/release/pulse
    cargo generate-rpm
    ln -sf $(find target -name "*.rpm" | head -1) pulse.rpm

reinstall:
    just rpm
    sudo dnf reinstall pulse.rpm
