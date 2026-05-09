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

# Lint code (fmt + clippy + bevy_lint)
[group('lint')]
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    bevy_lint --all-targets --all-features

# Run only bevy_lint
[group('lint')]
bevy-lint *args:
    bevy_lint {{args}}

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

# Run with dynamic linking + hotpatching for fast iteration
[group('run')]
dev *args:
    cargo run --features dev {{args}}

# Run with dev + bevy dev tools (inspector, picking debug, etc.)
[group('run')]
dev-tools *args:
    cargo run --features dev-tools {{args}}

# Watch for changes
[group('run')]
watch:
    bacon
