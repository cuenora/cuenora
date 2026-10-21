# Security Policy

Security and privacy are central to Cuenora because personal financial information is sensitive. Thank you for helping us identify and address potential vulnerabilities responsibly.

## Project status and supported versions

Cuenora is in early development. There are no stable releases, and the software is **not ready for production use or real financial data**.

| Version | Security support |
| --- | --- |
| `main` (development) | Reviewed and addressed on a best-effort basis |
| Unreleased builds and experimental branches | No guaranteed support |
| Third-party forks or modified deployments | Not maintained by the Cuenora project |

This is a volunteer-maintained project. No response time, remediation deadline, or security service-level agreement is guaranteed.

## Reporting a vulnerability

**Please do not disclose security vulnerabilities in public GitHub issues, discussions, or pull requests.**

The preferred reporting channel is **GitHub private vulnerability reporting**:

1. Open the [Cuenora security advisories page](https://github.com/cuenora/cuenora/security/advisories).
2. Select **Report a vulnerability** if that option is available.
3. Submit the issue privately with sufficient detail for maintainers to investigate.

If private vulnerability reporting is not enabled or cannot be accessed, open a public issue **only to request a private reporting channel**. Do not include exploit details, credentials, private data, or a proof of concept in that issue.

A useful private report includes:

- A short summary of the vulnerability and its potential impact.
- Affected component, version or commit, and relevant configuration.
- Clear reproduction steps and a minimal proof of concept using synthetic data.
- Security assumptions, preconditions, and any suggested mitigation.

Please avoid submitting real user records or secrets, even through the private reporting form.

## Responsible testing and disclosure

- Test only local installations, accounts, and infrastructure that you own or are explicitly authorized to assess.
- Do not access, alter, or exfiltrate another person's data.
- Do not perform destructive testing, denial-of-service attacks, or automated scanning of deployments without permission.
- Allow maintainers a reasonable opportunity to assess and address a report before public disclosure, and coordinate disclosure where feasible.

The maintainers may request clarification, reproduce the issue, prepare a fix and tests, and publish a security advisory when appropriate. The project cannot promise a particular remediation timeline.

## Security priorities

The following areas deserve particular care as Cuenora develops:

- Authentication, session management, and credential storage.
- Authorization and strict isolation of each user's accounts and transactions.
- Monetary correctness, transfer atomicity, and protection against duplicate operations.
- Input validation, data integrity, and safe database migrations.
- Protection of secrets in mobile apps, backend services, and CI workflows.
- Encryption in transit, secure deployment defaults, and dependency security.
- Privacy-preserving diagnostics, logs, backups, and data export.

## Self-hosted deployments

When self-hosting becomes supported, operators will be responsible for securing their own infrastructure, access controls, secret management, backups, and updates. Documentation will be expanded before a production-ready release.

For general development questions and non-sensitive bugs, please use [GitHub Issues](https://github.com/cuenora/cuenora/issues).
