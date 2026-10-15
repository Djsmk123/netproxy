# NetProxy Development Guide

This guide covers everything you need to know to contribute to NetProxy.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Building](#building)
- [Testing](#testing)
- [Code Style](#code-style)
- [Contributing](#contributing)
- [Debugging](#debugging)

## Getting Started

### Prerequisites

- **Rust**: 1.70 or later
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **macOS**: 13.0+ (for SwiftUI app)

- **Git**: For version control

### Clone the Repository

```bash
git clone https://github.com/yourusername/netproxy.git
cd netproxy
```

## Development Setup

### Rust Environment

1. **Install Rust toolchain**:
   ```bash
   rustup update stable
   rustup default stable
   ```

2. **Install development tools**:
   ```bash
   # Clippy (linter)
   rustup component add clippy

   # Rustfmt (formatter)
   rustup component add rustfmt
   ```

3. **Verify installation**:
   ```bash
   cargo --version
   cargo clippy --version
   cargo fmt --version
   ```

### IDE Setup

#### VS Code

Recommended extensions:
- **rust-analyzer**: IntelliSense for Rust
- **CodeLLDB**: Debugger
- **Better TOML**: TOML syntax highlighting
- **Even Better TOML**: Enhanced TOML support

Settings:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

#### CLion / RustRover

- Enable Rust plugin
- Configure auto-format with rustfmt
- Enable clippy lints

### Environment Variables

For development, you may want to set:

```bash
# Enable debug logging
export RUST_LOG=debug

# Use a test data directory
export NP_DATA_DIR=/tmp/netproxy-dev
```

## Project Structure

```
netproxy/
├── Cargo.toml                # Workspace definition
├── Cargo.lock               # Locked dependencies
├── README.md                # Project README
├── docs/                    # Documentation
│   ├── architecture.md
│   ├── control-api.md
│   ├── development.md
│   └── mock-system.md
├── app/                     # SwiftUI macOS app
│   └── NetProxy.xcodeproj
├── crates/                  # Rust crates
│   ├── np-model/           # Shared types
│   ├── np-engine/          # Proxy engine
│   ├── np-store/           # SQLite storage
│   ├── np-mock/            # Mock runtime
│   └── netproxyd/          # Daemon
└── harness/                 # Test infrastructure
```

### Crate Dependencies

```
netproxyd
  ├─→ np-engine
  │     └─→ np-model
  ├─→ np-store
  │     └─→ np-model
  └─→ np-mock
        ├─→ np-model
        └─→ np-engine
```

## Building

### Build All Crates

```bash
# Debug build (fast, unoptimized)
cargo build

# Release build (slow, optimized)
cargo build --release
```

### Build Specific Crate

```bash
cargo build --package netproxyd
cargo build --package np-mock
```

### Build Binary

```bash
# Debug binary
cargo build --bin netproxyd

# Release binary
cargo build --release --bin netproxyd

# Binary location
./target/debug/netproxyd       # Debug
./target/release/netproxyd     # Release
```

### Check Without Building

```bash
# Fast check (no codegen)
cargo check

# Check all targets
cargo check --all-targets
```

## Testing

### Run All Tests

```bash
# Run all tests in workspace
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_name
```

### Test Specific Crate

```bash
# Unit tests
cargo test --package np-mock

# Integration tests
cargo test --package netproxyd --test mocks

# Doc tests
cargo test --doc
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html

# View report
open tarpaulin-report.html
```

### Running Tests Sequentially

Some integration tests may conflict when run in parallel:

```bash
cargo test --package netproxyd -- --test-threads=1
```

### Test Organization

- **Unit tests**: Inside `src/` files in `#[cfg(test)] mod tests`
- **Integration tests**: In `tests/` directory
- **Test fixtures**: In test crates or `tests/fixtures/`

## Code Style

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --all-targets --all-features

# Fix warnings automatically
cargo clippy --all-targets --all-features --fix
```

### Conventions

#### Naming

- **Crates**: Lowercase with hyphens (`np-mock`)
- **Modules**: Lowercase with underscores (`pattern_matcher`)
- **Types**: PascalCase (`MockResolver`)
- **Functions**: snake_case (`load_config`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_EVENTS`)

#### Error Handling

Use `thiserror` for error types:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MockError {
    #[error("rule not found: {0}")]
    RuleNotFound(String),
    #[error("fixture error: {0}")]
    Fixture(#[from] FixtureError),
}
```

#### Documentation

- Public items MUST have doc comments
- Use `///` for doc comments
- Include examples for public APIs

```rust
/// Resolves mock responses for incoming requests.
///
/// # Examples
///
/// ```
/// let resolver = RuleMockResolver::new(config, inner)?;
/// let decision = resolver.resolve(&request);
/// ```
pub struct RuleMockResolver {
    // ...
}
```

#### Comments

- Use `//` for implementation comments
- Explain WHY, not WHAT
- Keep comments up-to-date with code

```rust
// Use SHA-256 instead of MD5 for security
let hash = sha256(&content);
```

## Contributing

### Workflow

1. **Fork the repository** on GitHub

2. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make changes**:
   - Write code
   - Add tests
   - Update documentation

4. **Commit changes**:
   ```bash
   git add .
   git commit -m "Add feature: description"
   ```

5. **Push to your fork**:
   ```bash
   git push origin feature/my-feature
   ```

6. **Create a Pull Request** on GitHub

### Commit Messages

Follow conventional commits:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Maintenance

**Examples**:
```
feat(mock): add regex path matching

Add support for regex patterns in path matching.
This allows more flexible rule definitions.

Closes #123
```

```
fix(engine): handle empty response bodies

Empty bodies were causing panics. Now properly
handled with zero-length Bytes.
```

### Pull Request Guidelines

1. **Title**: Clear, concise description
2. **Description**: Explain WHAT and WHY
3. **Tests**: Include tests for new features
4. **Documentation**: Update relevant docs
5. **Changelog**: Add entry if user-facing change

### Code Review

- Be respectful and constructive
- Focus on code, not person
- Explain reasoning for suggestions
- Be open to feedback

## Debugging

### Logging

NetProxy uses `tracing` for structured logging:

```rust
use tracing::{debug, info, warn, error};

info!("starting engine on {}", addr);
debug!(rule_id = %id, "rule matched");
warn!("fixture not found: {}", file);
error!(error = %e, "failed to load config");
```

**Enable logging**:
```bash
# All debug logs
RUST_LOG=debug cargo run

# Specific crate
RUST_LOG=np_mock=debug cargo run

# Multiple levels
RUST_LOG=netproxyd=info,np_mock=debug cargo run
```

### Debugging with LLDB

#### VS Code

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug netproxyd",
      "cargo": {
        "args": ["build", "--bin=netproxyd"]
      },
      "args": ["--data-dir", "/tmp/netproxy-debug"],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

#### Command Line

```bash
# Build with debug symbols
cargo build --bin netproxyd

# Run with lldb
lldb ./target/debug/netproxyd

# Set breakpoint
(lldb) b netproxyd::daemon::start
(lldb) run --data-dir /tmp/test
```

### Testing the Proxy

#### Manual Testing

1. **Start daemon**:
   ```bash
   cargo run --bin netproxyd -- --data-dir /tmp/test
   ```

2. **In another terminal, use curl**:
   ```bash
   curl -x http://127.0.0.1:8899 http://example.com
   ```

#### With Mock Rules

1. **Create test mocks**:
   ```bash
   mkdir -p /tmp/test/fixtures
   cat > /tmp/test/mocks.yaml <<EOF
   version: 1
   fixtures_dir: ./fixtures
   rules:
     - id: test
       priority: 100
       enabled: true
       match:
         host: "test.local"
       action:
         type: respond
         status: 200
         body:
           inline: "It works!"
   EOF
   ```

2. **Test**:
   ```bash
   curl -x http://127.0.0.1:8899 http://test.local
   # Response: It works!
   ```

### Profiling

#### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin netproxyd

# View flamegraph.svg
open flamegraph.svg
```

#### Memory Profiling

```bash
# Install valgrind (macOS)
brew install valgrind

# Run with valgrind
valgrind --leak-check=full ./target/debug/netproxyd
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench --bench my_bench
```

## Common Tasks

### Adding a New Crate

1. **Create crate**:
   ```bash
   cargo new --lib crates/np-newcrate
   ```

2. **Update workspace `Cargo.toml`**:
   ```toml
   members = [
       # ...
       "crates/np-newcrate"
   ]
   ```

3. **Add dependencies**:
   ```toml
   # crates/np-newcrate/Cargo.toml
   [dependencies]
   np-model = { path = "../np-model" }
   ```

### Adding a New Test

#### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

#### Integration Test

Create `tests/my_test.rs`:

```rust
use netproxyd::Daemon;

#[tokio::test]
async fn test_integration() {
    let daemon = setup_daemon().await;
    // Test logic
}
```

### Updating Dependencies

```bash
# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Update specific dependency
cargo update -p serde
```

### Cleaning Build Artifacts

```bash
# Clean target directory
cargo clean

# Remove lock file
rm Cargo.lock

# Clean and rebuild
cargo clean && cargo build
```

## Troubleshooting

### Build Errors

**Problem**: `error: linking with cc failed`

**Solution**: Install Xcode Command Line Tools:
```bash
xcode-select --install
```

---

**Problem**: `error: could not compile due to previous error`

**Solution**: 
1. Check Rust version: `rustc --version`
2. Update: `rustup update`
3. Clean: `cargo clean`

### Test Failures

**Problem**: Integration tests fail with "Address already in use"

**Solution**: Run tests sequentially:
```bash
cargo test -- --test-threads=1
```

---

**Problem**: Tests timeout

**Solution**: Increase timeout or run fewer parallel tests:
```bash
cargo test -- --test-threads=4
```

### Runtime Issues

**Problem**: Daemon fails to start

**Solution**: Check logs and permissions:
```bash
RUST_LOG=debug cargo run --bin netproxyd -- --data-dir /tmp/test
```

---

**Problem**: Proxy not capturing traffic

**Solution**: 
1. Verify proxy settings in client
2. Check daemon status
3. Ensure CA certificate is trusted

## Resources

### Documentation

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Rustls Documentation](https://docs.rs/rustls/)

### Community

- GitHub Issues: Bug reports and feature requests
- GitHub Discussions: Questions and general discussion

---

**Happy hacking!** 🦀
