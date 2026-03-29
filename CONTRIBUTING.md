# Contributing to Rainy Cowork

Thank you for your interest in contributing to Rainy Cowork! We welcome contributions from the community to help make this open-source AI coworking tool even better.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Submitting Changes](#submitting-changes)
- [Reporting Issues](#reporting-issues)
- [Feature Requests](#feature-requests)

## 🤝 Code of Conduct

This project follows a code of conduct to ensure a welcoming environment for all contributors. By participating, you agree to:

- Be respectful and inclusive
- Focus on constructive feedback
- Accept responsibility for mistakes
- Show empathy towards other contributors
- Help create a positive community

## 🚀 How to Contribute

### Types of Contributions

- **Bug fixes**: Fix issues in the codebase
- **Features**: Add new functionality
- **Documentation**: Improve docs, README, etc.
- **Tests**: Add or improve test coverage
- **UI/UX**: Enhance user interface and experience

### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## 🛠️ Development Setup

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Tauri CLI

### Setup Steps

```bash
# Clone your fork
git clone https://github.com/yourusername/rainy-cowork.git
cd rainy-cowork

# Install dependencies
npm install

# Start development server
npm run tauri dev
```

### Project Structure

```
rainy-cowork/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/             # React hooks
│   └── types/             # TypeScript types
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── ai/            # AI provider integrations
│   │   ├── commands/      # Tauri commands
│   │   └── services/      # Backend services
│   └── Cargo.toml
└── package.json
```

## 📝 Submitting Changes

### Pull Request Process

1. Ensure your code follows the project's style guidelines
2. Update documentation if needed
3. Add tests for new features
4. Ensure all tests pass
5. Update the changelog if applicable

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add new AI provider integration
fix: resolve file permission issue
docs: update installation instructions
```

### Code Style

- **Frontend**: Follow React and TypeScript best practices
- **Backend**: Follow Rust coding standards
- **General**: Use consistent naming and formatting

## 🐛 Reporting Issues

When reporting bugs:

1. Use the issue template
2. Include detailed steps to reproduce
3. Provide system information (OS, versions)
4. Attach relevant logs or screenshots
5. Check for existing issues first

## 💡 Feature Requests

For new features:

1. Check existing issues and discussions
2. Create a detailed proposal
3. Explain the use case and benefits
4. Consider implementation complexity

## 🧪 Testing

- Run frontend tests: `npm test`
- Run backend tests: `cargo test`
- Test across different platforms when possible

## 📚 Documentation

- Keep README.md up to date
- Document new features
- Update API docs for backend changes

## 🙏 Recognition

Contributors will be acknowledged in the project's changelog and README. Significant contributions may be recognized in future releases.

Thank you for contributing to Rainy Cowork! 🎉
