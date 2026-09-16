# ycallr-core

High-performance API execution engine behind ycallr: YAML/OpenAPI → protobuf AOT compilation, shared call engine, HTTP client, C FFI, and WASM bindings.

## Features

- YAML API profiles with nested commands, auth, env templates, response messages
- OpenAPI 3.x import (`openapi` feature)
- SSRF-safe HTTP client (no redirects, host validation at ingest)
- Secrets via `${ENV_VAR}` — never stored in compiled profiles

## Quick example profile

See [`examples/github_api.yaml`](examples/github_api.yaml):

```yaml
env:
  - name: GITHUB_TOKEN
    required: true
auth:
  github:
    type: bearer
    token: ${GITHUB_TOKEN}
commands:
  list-issues:
    endpoint: /repos/{owner}/{repo}/issues
    method: GET
    auth: github
```

## Development

```bash
make build
make test
make ci                 # fmt + clippy + test + coverage (≥85%)
make release-check
```

## Documentation

Wiki: [API Profiles](https://github.com/Coditary/ycallr-core/wiki/API-Profiles), [C FFI](https://github.com/Coditary/ycallr-core/wiki/C-FFI), [OpenAPI Import](https://github.com/Coditary/ycallr-core/wiki/OpenAPI-Import).
