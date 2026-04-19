set dotenv-load

default:
    just --list

# Build the project
[group('build')]
build *args:
    cargo build {{args}}

# Build release
[group('build')]
build-release:
    cargo build --release

# Run tests
[group('test')]
test *args:
    cargo nextest run {{args}}

# Lint code
[group('lint')]
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
[group('lint')]
fmt:
    cargo fmt --all
    taplo fmt

# Clean build artifacts
[group('clean')]
clean:
    cargo clean

# Run the game
[group('run')]
run *args:
    cargo run {{args}}

# Watch for changes
[group('run')]
watch:
    bacon
