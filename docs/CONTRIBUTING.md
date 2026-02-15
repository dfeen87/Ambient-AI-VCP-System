# Contributing to Ambient AI + VCP System

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on what is best for the community

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/Ambient-AI-VCP-System.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Run formatting: `cargo fmt`
7. Commit your changes: `git commit -m "Add your feature"`
8. Push to your fork: `git push origin feature/your-feature-name`
9. Create a Pull Request

## Development Guidelines

### Code Style

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write unit tests for new functionality
- Add documentation for public APIs

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p ambient-node

# Run tests with logging
RUST_LOG=debug cargo test
```

### Commit Messages

- Use clear, descriptive commit messages
- Start with a verb in present tense (Add, Fix, Update, etc.)
- Keep first line under 72 characters
- Add details in the body if needed

Example:
```
Add telemetry collection for GPU metrics

- Implement GPU temperature monitoring
- Add VRAM usage tracking
- Update health score calculation
```

## Areas for Contribution

### High Priority

- Real ZK proof implementation (RISC Zero or Plonky2)
- P2P networking layer (libp2p)
- Production WASM runtime integration
- Security audit and hardening

### Medium Priority

- Advanced privacy-preserving techniques
- Enhanced ZK proof systems
- Web dashboard enhancements
- Performance optimizations

### Good First Issues

- Documentation improvements
- Additional unit tests
- Example applications
- Bug fixes

## Pull Request Process

1. Update documentation if needed
2. Add tests for new functionality
3. Ensure all tests pass
4. Update README.md if needed
5. Request review from maintainers

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
