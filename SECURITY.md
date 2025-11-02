# Security Policy

## Supported Versions

Currently, OtterLang is in **Early Access (v0.1.0)**. Security updates are provided for the latest version.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in OtterLang, please report it responsibly:

1. **Do NOT** open a public GitHub issue
2. Message security details to our discord: https://discord.gg/y3b4QuvyFk
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will:
- Acknowledge receipt within 48 hours
- Provide an initial assessment within 7 days
- Keep you informed of progress
- Credit you in security advisories (unless you prefer otherwise)

## Security Considerations

### Known Limitations

As an experimental language, OtterLang has some security considerations:

1. **FFI System**: FFI bridges load dynamic libraries. Only use trusted Rust crates.
2. **Memory Safety**: OtterLang uses reference counting and garbage collection. Memory safety bugs may exist.
3. **Runtime**: The JIT runtime executes generated code. Untrusted code execution should be avoided.
4. **No Sandboxing**: Currently, OtterLang programs run with full system permissions.

### Best Practices

- **Validate Input**: Always validate user input and external data
- **Use Trusted FFI**: Only import Rust crates from trusted sources
- **Error Handling**: Use proper error handling (nil checks, etc.)
- **Keep Updated**: Use the latest version when available

## Security Updates

Security updates will be released as patch versions (0.1.x) and announced in:
- GitHub Releases
- Security Advisories
- CHANGELOG.md

## Acknowledgments

We thank security researchers who responsibly disclose vulnerabilities.

