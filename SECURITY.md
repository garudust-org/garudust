# Security Policy

## Supported Versions

Only the latest commit on `main` is actively supported. There are no versioned releases with long-term security backports at this time.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report security issues privately through one of these channels:

1. **GitHub Security Advisories (preferred)** — [Open a private advisory](https://github.com/ninenox/garudust/security/advisories/new). This keeps the report confidential until a fix is ready.
2. **Email** — `nashnox15@gmail.com` with subject `[SECURITY] Garudust — <short description>`.

### What to include

- A description of the vulnerability and its potential impact
- Steps to reproduce (command, config, request payload, etc.)
- The version / commit you tested against
- Any suggested fix or mitigation (optional but appreciated)

### Response timeline

| Milestone | Target |
|-----------|--------|
| Acknowledgement | Within 72 hours |
| Initial assessment | Within 7 days |
| Fix / advisory published | Depends on severity and complexity |

## Threat Model

Garudust is designed for **self-hosted, trusted-network deployment**. Key assumptions:

- The HTTP gateway (`/chat*`) should be protected by `GARUDUST_API_KEY` before exposing it to the internet.
- The `terminal` tool executes shell commands — do not expose Garudust to untrusted users without setting `--approval-mode deny` or `smart`.
- Platform adapter credentials (Telegram token, Slack tokens, etc.) grant the agent access to those platforms as a bot. Treat them like API keys.
- MCP server subprocesses are launched with the agent's full OS permissions. Only connect trusted MCP servers.

## Security Controls

| Control | Configuration |
|---------|---------------|
| HTTP authentication | `GARUDUST_API_KEY=<token>` |
| Command approval | `GARUDUST_APPROVAL_MODE=smart` (default) or `deny` |
| Per-IP rate limiting | `GARUDUST_RATE_LIMIT=60` (req/min, opt-in) |
| SSRF protection | Built-in — blocks private IPs and cloud metadata endpoints |
| File path sandboxing | Reads/writes restricted to cwd and home directory |
| Secrets isolation | `.env` values never written to process environment |
