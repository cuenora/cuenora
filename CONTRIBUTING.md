# Contributing to Cuenora

Thank you for your interest in Cuenora! Contributions of all sizes are welcome, including bug reports, documentation, tests, design discussions, and code.

Cuenora is an early-stage, non-commercial, open-source personal finance project. Its architecture, APIs, and development workflows are still being established. Please keep that in mind when proposing changes.

## Before you contribute

- Read the [README](README.md) for the project's vision and planned architecture.
- Search existing issues and pull requests before opening a new one.
- Open an issue to discuss significant features, breaking changes, or architectural decisions before investing in a large implementation.
- Keep proposals focused. Small, independently reviewable changes are preferred.
- For suspected vulnerabilities, follow the [security policy](SECURITY.md) instead of opening a public issue.

## Ways to contribute

### Report a bug

Use [GitHub Issues](https://github.com/cuenora/cuenora/issues) and include:

- A concise description of the problem.
- Steps to reproduce it, when possible.
- Expected and actual behavior.
- Relevant platform, OS, toolchain, and version information.
- Logs or screenshots with all secrets and personal or financial information removed.

If a component has not been implemented yet, describe the problem as a proposal or design discussion rather than a working-software bug.

### Suggest a feature

Explain the user problem, the proposed behavior, and any important edge cases. For financial features, consider multiple accounts, currencies, dates, rounding, cancellations, and the difference between planned and completed transactions.

### Improve documentation or tests

Documentation fixes, reproducible test cases, and clearer examples are valuable contributions, especially while the project is being established.

## Pull request workflow

1. Fork the repository, or create a branch if you have write access.
2. Create a descriptive branch, such as `feat/account-model`, `fix/transfer-validation`, or `docs/contributing-guide`.
3. Implement one focused change and add or update relevant tests.
4. Run the applicable formatters, linters, and tests for the components you changed. See the component documentation once the build systems are initialized.
5. Update documentation when behavior, interfaces, or setup instructions change.
6. Open a pull request against `main`, explain the motivation, and link related issues.
7. Address review comments and wait for required checks to pass before merging.

Please do not push directly to `main`. The repository uses pull requests to keep changes traceable.

### Pull request checklist

- [ ] The change has a clear purpose and stays within the agreed scope.
- [ ] Tests were added or updated where relevant, or the reason for not adding them is explained.
- [ ] Applicable formatting, linting, and tests pass locally.
- [ ] Documentation and API contracts were updated if needed.
- [ ] No credentials, tokens, private keys, real financial records, or personal data are included.
- [ ] The pull request describes any migrations, compatibility changes, or follow-up work.

## Engineering principles

### Correctness of financial data

Financial correctness takes priority over convenience:

- Do not use binary floating-point types to represent monetary amounts in accounting logic.
- Make rounding, currency, time zone, and date assumptions explicit.
- Treat transfers between a user's own accounts as movements of funds, not income or expenses.
- Keep scheduled or projected amounts separate from completed transactions.
- Use database transactions where an operation must succeed or fail atomically.
- Include tests for edge cases such as insufficient funds policies, duplicate requests, leap years, and month-end recurring schedules when relevant.

These are design requirements, not claims that the features already exist.

### Architecture and code style

The planned repository contains a Rust backend, Kotlin Multiplatform shared mobile logic, and native Android and iOS interfaces.

- Prefer simple, explicit, maintainable implementations over unnecessary abstraction.
- Keep domain logic independent of HTTP, databases, and UI frameworks where practical.
- Preserve clear boundaries between platform-specific and shared mobile code.
- Avoid introducing dependencies without explaining their purpose.
- Follow existing conventions and use the formatters and analyzers adopted by each component.
- Keep public API changes documented and consider backward compatibility for mobile clients.

Build commands and toolchain requirements will be documented as each component is introduced. Please do not assume an unfinished component can already be built or tested.

### Tests

The intended test layers are:

- Rust unit tests for domain rules and application logic.
- PostgreSQL integration tests for persistence and transactional behavior.
- Kotlin Multiplatform `commonTest` for shared mobile logic.
- Platform-specific tests for native integrations and UI behavior.
- Cross-platform mobile end-to-end flows where Android and iOS behavior matches.

Changes to money handling, permissions, or account isolation should include focused regression tests whenever possible.

## Security and privacy

Never commit or attach real financial records, credentials, session tokens, private keys, `.env` secrets, or production database dumps. Use synthetic fixtures and redacted logs. Do not test against accounts or deployments you do not own or have explicit permission to assess.

Report potential security vulnerabilities privately using the process in [SECURITY.md](SECURITY.md).

## Communication

Be respectful, constructive, and patient. Assume good intent, provide actionable feedback, and welcome contributors with different levels of experience. English is the primary language for repository documentation, issues, pull requests, and code comments.

As a small volunteer project, Cuenora does not guarantee response or review times.

## License

By submitting a contribution, you agree that your contribution may be distributed under the project's [MIT License](LICENSE). Only submit code, assets, or documentation that you have the right to contribute.
