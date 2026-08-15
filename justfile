# Default recipe
default: build

# Build in debug mode
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# Run the cosmic-rdp application
run:
    env RUST_BACKTRACE=1 cargo run -p cosmic-rdp

# Run all automated tests
test:
    cargo test --workspace

# Run clippy linter and check
check:
    cargo check --workspace

# Install to system or prefix directory
install:
    install -Dm0755 target/release/cosmic-rdp $(DESTDIR)/usr/bin/cosmic-rdp
    install -Dm0644 crates/cosmic-rdp/resources/app.desktop $(DESTDIR)/usr/share/applications/dev.cosmic.Rdp.desktop
    install -Dm0644 crates/cosmic-rdp/resources/app.metainfo.xml $(DESTDIR)/usr/share/metainfo/dev.cosmic.Rdp.metainfo.xml
    install -Dm0644 crates/cosmic-rdp/resources/icons/hicolor/scalable/apps/icon.svg $(DESTDIR)/usr/share/icons/hicolor/scalable/apps/dev.cosmic.Rdp.svg
