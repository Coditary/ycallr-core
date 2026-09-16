# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in ycallr-core, please report it responsibly:

1. **Do not** open a public GitHub issue for security-sensitive findings.
2. Email the maintainer or open a private security advisory on GitHub if available.
3. Include a clear description, steps to reproduce, and potential impact.

We aim to acknowledge reports within 72 hours and provide a fix or mitigation plan as soon as possible.

## Security Design Notes

- API profile URLs are validated at ingest time to block SSRF targets (loopback, private IPs, metadata hosts).
- HTTP redirects are disabled by default.
- Auth secrets are resolved from environment variables at runtime, not stored in compiled profiles.
- Cookie auth values reject injection characters (`;`, CRLF).
