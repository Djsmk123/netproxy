# Contributing to NetProxy

Thank you for your interest in contributing to NetProxy! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Reporting Issues](#reporting-issues)
- [Community](#community)

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive experience for everyone. We expect all contributors to:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, trolling, or discriminatory comments
- Personal attacks or insults
- Publishing others' private information
- Other conduct which could reasonably be considered inappropriate

## Getting Started

1. **Read the documentation**:
   - [README.md](README.md) - Project overview
   - [Architecture](docs/architecture.md) - System design
   - [Development Guide](docs/development.md) - Setup and workflow

2. **Set up your development environment**:
   ```bash
   # Clone your fork
   git clone https://github.com/YOUR_USERNAME/netproxy.git
   cd netproxy

   # Add upstream remote
   git remote add upstream https://github.com/original/netproxy.git

   # Build the project
   cargo build
   
   # Run tests
   cargo test --workspace
   ```

3. **Find an issue to work on**:
   - Look for issues labeled `good first issue` or `help wanted`
   - Check the [roadmap](README.md#roadmap) for planned features
   - Ask questions in issues or discussions

## How to Contribute

### Types of Contributions

We welcome many types of contributions:

- **Bug fixes**: Fix issues or unexpected behavior
- **Features**: Implement new functionality
- **Documentation**: Improve or add documentation
- **Tests**: Add test coverage
- **Performance**: Optimize code
- **Refactoring**: Improve code quality
- **Examples**: Add usage examples

### Before You Start

1. **Check existing issues**: Ensure your idea isn't already being worked on
2. **Create an issue**: For significant changes, create an issue first to discuss
3. **Get feedback**: Wait for maintainer feedback before starting work
4. **Keep it focused**: One feature/fix per pull request

## Development Workflow

### 1. Fork and Clone

```bash
# Fork the repository on GitHub
# Clone your fork
git clone https://github.com/YOUR_USERNAME/netproxy.git
cd netproxy

# Add upstream
git remote add upstream https://github.com/original/netproxy.git
```

### 2. Create a Branch

```bash
# Update your fork
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/my-feature
```

**Branch naming**:
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation
- `test/description` - Tests
- `refactor/description` - Refactoring

### 3. Make Changes

1. **Write code** following [coding standards](#coding-standards)
2. **Add tests** for new functionality
3. **Update documentation** as needed
4. **Run tests** to ensure nothing breaks

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features

# Run tests
cargo test --workspace
```

### 4. Commit Changes

Write clear, descriptive commit messages:

```bash
# Stage changes
git add .

# Commit with message
git commit -m "feat(mock): add regex path matching

Add support for regex patterns in path matching.
This allows more flexible rule definitions.

Closes #123"
```

**Commit message format**:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

### 5. Push and Create PR

```bash
# Push to your fork
git push origin feature/my-feature

# Create pull request on GitHub
```

## Coding Standards

### Rust Code Style

1. **Format with rustfmt**:
   ```bash
   cargo fmt --all
   ```

2. **Lint with clippy**:
   ```bash
   cargo clippy --all-targets --all-features
   ```

3. **Follow Rust conventions**:
   - Use snake_case for functions and variables
   - Use PascalCase for types
   - Use SCREAMING_SNAKE_CASE for constants
   - Prefer explicit types when clarity benefits

### Documentation

1. **Public items must have doc comments**:
   ```rust
   /// Resolves mock responses for incoming requests.
   ///
   /// # Arguments
   ///
   /// * `req` - The incoming request to match
   ///
   /// # Returns
   ///
   /// A `MockDecision` indicating how to handle the request
   ///
   /// # Examples
   ///
   /// ```
   /// let decision = resolver.resolve(&request);
   /// ```
   pub fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
       // ...
   }
   ```

2. **Keep documentation up-to-date** with code changes

3. **Add examples** for complex functionality

### Testing

1. **Write unit tests** for new code:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_feature() {
           // Arrange
           let input = setup();

           // Act
           let result = function_under_test(input);

           // Assert
           assert_eq!(result, expected);
       }
   }
   ```

2. **Write integration tests** for significant features

3. **Aim for high coverage** (but quality > quantity)

4. **Test edge cases** and error conditions

### Error Handling

1. **Use `thiserror` for errors**:
   ```rust
   #[derive(Debug, Error)]
   pub enum MyError {
       #[error("not found: {0}")]
       NotFound(String),
       #[error("io error: {0}")]
       Io(#[from] std::io::Error),
   }
   ```

2. **Provide context** in error messages

3. **Use `Result` for fallible operations**

4. **Don't panic** in library code (except for bugs)

## Pull Request Process

### Before Submitting

- [ ] Code builds without warnings
- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] Clippy checks pass (`cargo clippy`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if user-facing)

### PR Description Template

```markdown
## Description
Brief description of changes

## Motivation
Why is this change needed?

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Related Issues
Closes #123
Related to #456

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
```

### Review Process

1. **Maintainer review**: A maintainer will review your PR
2. **Feedback**: Address review comments
3. **Approval**: PR approved by maintainer
4. **Merge**: PR merged to main branch

### After Merge

1. **Delete your branch**:
   ```bash
   git branch -d feature/my-feature
   git push origin --delete feature/my-feature
   ```

2. **Update your fork**:
   ```bash
   git checkout main
   git pull upstream main
   git push origin main
   ```

## Reporting Issues

### Bug Reports

When reporting bugs, include:

1. **Description**: What happened vs. what you expected
2. **Steps to reproduce**:
   ```
   1. Start daemon with config X
   2. Send request Y
   3. Observe error Z
   ```
3. **Environment**:
   - OS version
   - Rust version
   - NetProxy version
4. **Logs**: Include relevant logs (`RUST_LOG=debug`)
5. **Configuration**: Relevant config files (sanitized)

### Feature Requests

When requesting features, include:

1. **Use case**: What problem does this solve?
2. **Proposed solution**: How should it work?
3. **Alternatives**: What alternatives did you consider?
4. **Additional context**: Mockups, examples, etc.

## Community

### Getting Help

- **Documentation**: Check [docs/](docs/)
- **Issues**: Search existing issues
- **Discussions**: Ask questions in GitHub Discussions

### Staying Updated

- **Watch the repository** for notifications
- **Read the CHANGELOG** for release notes
- **Follow the roadmap** for upcoming features

### Recognition

Contributors are recognized in:
- Git commit history
- Release notes
- Project README (for significant contributions)

## Development Tips

### Fast Iteration

```bash
# Quick check (no build)
cargo check

# Build specific crate
cargo build --package np-mock

# Run specific test
cargo test test_name

# Watch mode (requires cargo-watch)
cargo watch -x test
```

### Debugging

```bash
# Enable debug logs
RUST_LOG=debug cargo run

# Run with debugger (VS Code)
# Use launch.json configuration
```

### Performance

```bash
# Release build for benchmarking
cargo build --release

# Profile with flamegraph
cargo flamegraph --bin netproxyd
```

## Questions?

If you have questions not covered here:

1. Check the [Development Guide](docs/development.md)
2. Search existing [issues](https://github.com/yourusername/netproxy/issues)
3. Ask in [Discussions](https://github.com/yourusername/netproxy/discussions)
4. Email the maintainer: djsmk123@gmail.com

---

**Thank you for contributing to NetProxy!** 🎉
