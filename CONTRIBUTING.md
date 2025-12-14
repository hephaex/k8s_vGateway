# Contributing to k8s_vGateway

Thank you for your interest in contributing to k8s_vGateway! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We welcome contributors of all backgrounds and experience levels.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A Kubernetes cluster (for integration testing)
- kubectl configured with cluster access

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/k8s_vGateway.git
   cd k8s_vGateway
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/hephaex/k8s_vGateway.git
   ```
4. Build the project:
   ```bash
   cargo build
   ```
5. Run tests:
   ```bash
   cargo test
   ```

## Development Workflow

### Branch Naming

Use descriptive branch names with prefixes:
- `feat/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test additions or modifications
- `chore/` - Maintenance tasks

Example: `feat/add-kong-gateway-support`

### Commit Messages

Follow conventional commit format:

```
<type>: <description>

[optional body]

[optional footer]
```

Types:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation
- `style` - Formatting (no code change)
- `refactor` - Code refactoring
- `test` - Adding tests
- `chore` - Maintenance

Examples:
```
feat: Add support for Kong Gateway
fix: Resolve timeout issue in health checks
docs: Update installation instructions
```

### Code Style

- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes with no warnings
- Follow Rust naming conventions
- Add documentation comments for public APIs

```bash
# Format code
cargo fmt

# Check for warnings
cargo clippy

# Run tests
cargo test
```

## Pull Request Process

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature
   ```

2. **Make your changes** and commit:
   ```bash
   git add .
   git commit -m "feat: Add your feature"
   ```

3. **Keep your branch updated**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

4. **Push to your fork**:
   ```bash
   git push origin feat/your-feature
   ```

5. **Open a Pull Request** with:
   - Clear title describing the change
   - Description of what and why
   - Link to related issues (if any)
   - Screenshots (for UI changes)

### PR Checklist

- [ ] Code compiles without errors
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation updated (if needed)
- [ ] Commit messages follow conventions

## Adding New Gateway Support

To add support for a new Gateway API implementation:

1. Add the gateway variant to `src/models/gateway.rs`
2. Implement installation logic in `src/deploy/installer.rs`
3. Add health check configuration in `src/deploy/health.rs`
4. Update tests in relevant modules
5. Add documentation to README.md

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests for specific module
cargo test module_name::
```

### Writing Tests

- Place unit tests in the same file as the code
- Use descriptive test names
- Test both success and failure cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_works() {
        // Arrange
        let input = "test";

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

## Reporting Issues

### Bug Reports

Include:
- Clear description of the bug
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version, K8s version)
- Relevant logs or error messages

### Feature Requests

Include:
- Clear description of the feature
- Use case and motivation
- Proposed implementation (if any)

## Getting Help

- Open an issue for questions
- Check existing issues before creating new ones
- Provide context and details in your questions

## License

By contributing, you agree that your contributions will be licensed under the GPL-3.0 License.

## Acknowledgments

Thank you to all contributors who help improve this project!
