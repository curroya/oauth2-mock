# oauth2-mock

[![ci](../../actions/workflows/ci.yml/badge.svg)](../../actions/workflows/ci.yml)

A tiny OpenID Connect provider for local development and end-to-end tests: the authorization-code flow with PKCE, signed RS256 ID tokens, and a one-click sign-in page.

| | oauth2-mock | Dex | mock-oauth2-server (JVM) |
|---|---|---|---|
| Image | ~52 MB | ~100 MB | ~350 MB |
| Idle memory | ~3 MiB | ~25 MB | ~150–300 MB |
| Sign in | Click a user, type any email, or pass `login_hint` | Email and password form | Form or `login_hint` |
| Users | Optional list; any email works | Static list with bcrypt hashes | Any username |
| Signing key | Stable across restarts | New on every boot | New on every boot |

## Quick start

```bash
docker run --rm -p 5556:5556 -e USERS=alice@example.test,bob@example.test curroya/oauth2-mock
```

Images for `linux/amd64` and `linux/arm64` are on [Docker Hub](https://hub.docker.com/r/curroya/oauth2-mock): `latest` follows `main`, and `sha-<commit>` pins one build.

Point your OIDC client at the issuer `http://localhost:5556`, with client id `local-client` and secret `local-secret`.

## How signing in works

| Request | Result |
|---|---|
| `GET /auth?…` without `login_hint` | The sign-in page: one button per configured user, plus an email field |
| `GET /auth?…&login_hint=alice@example.test` | Straight back to `redirect_uri` with `code` and `state`, no page |
| `POST /auth` | The sign-in page's submit; `login_hint` carries the email |

- Any email signs in, listed or not.
- Codes are single-use and live in memory.
- PKCE with `S256` is mandatory; a request without it gets `400`.
- A wrong `code_verifier`, a reused code, or a different `redirect_uri` gets `invalid_grant`.

## Configuration

Everything is an environment variable.

| Variable | Default | Purpose |
|---|---|---|
| `ISSUER` | `http://localhost:5556` | The exact `iss` claim and the base of the server-to-server endpoints; its path is where the routes are mounted |
| `BROWSER_BASE_URL` | `ISSUER` | Base of `authorization_endpoint`, for when the browser reaches the mock under another host than your backend |
| `CLIENT_ID` | `local-client` | The one accepted client |
| `CLIENT_SECRET` | `local-secret` | Its secret, sent as `client_secret_basic` or `client_secret_post` |
| `USERS` | none | Comma-separated emails to list on the sign-in page |
| `USERS_FILE` | none | Path to a JSON list of users |
| `KEY_SEED` | `oauth2-mock` | Seed of the deterministic RSA signing key |
| `TOKEN_LIFETIME_SECONDS` | `3600` | Lifetime of ID and access tokens |
| `PORT` | `5556` | Listen port |

The server refuses to start, naming the variable, when a value is invalid or `USERS_FILE` cannot be read.

### Users

`USERS_FILE` holds a JSON list. `name` and `id` are optional:

```json
[
  { "name": "Candidate", "email": "candidate@example.test", "id": "6f8d3c1e-0b2a-4f5c-9d7e-1a2b3c4d5e6a" },
  { "email": "empty@example.test" }
]
```

| Claim | From the list | Fallback |
|---|---|---|
| `sub` | `id` | SHA-256 of the trimmed, lowercased email: first 32 hex digits in UUID layout |
| `name` | `name` | The part of the email before `@` |
| `email` | `email`, trimmed and lowercased | — |
| `email_verified` | always `true` | — |

- `USERS` and `USERS_FILE` combine; the file wins when an email is in both.
- The fallback `sub` is the same on every restart and every machine, so an email always maps to the same account.

### Issuer and browser URLs

OIDC clients compare `iss` byte for byte. When your backend reaches the mock as `http://oidc:5556` inside Docker and the browser as `http://localhost:5556`, keep one issuer and move only the browser-facing endpoint:

```yaml
services:
  oidc:
    image: curroya/oauth2-mock
    ports: ["5556:5556"]
    environment:
      ISSUER: http://oidc:5556
      BROWSER_BASE_URL: http://localhost:5556
      CLIENT_ID: my-app
      CLIENT_SECRET: my-app-secret
      USERS_FILE: /users.json
    volumes:
      - ./users.json:/users.json:ro
```

## Endpoints

| Endpoint | Purpose |
|---|---|
| `GET {ISSUER}/.well-known/openid-configuration` | Discovery document |
| `GET {ISSUER}/keys` | JWKS with the one RS256 key |
| `GET`/`POST {BROWSER_BASE_URL}/auth` | Authorization endpoint and sign-in page |
| `POST {ISSUER}/token` | `authorization_code` grant |
| `GET {ISSUER}/healthz` | `ok` |

The token response carries `access_token` (random, opaque), `token_type: Bearer`, `expires_in` and `id_token`. The ID token carries `iss`, `sub`, `aud`, `iat`, `exp`, `nonce` when one was sent, `email`, `email_verified` and `name`.

## End-to-end tests

Click a configured user on the sign-in page:

```ts
await page.getByRole('button', { name: /candidate@example\.test/ }).click()
```

Or type any email, which needs no configuration:

```ts
await page.getByLabel('Email').fill('worker-0@example.test')
await page.getByRole('button', { name: 'Sign in' }).click()
```

Each email is a stable identity, so give every parallel worker its own address. Backends that forward `login_hint` to the provider skip the page entirely.

## Not supported

- Refresh tokens, `client_credentials`, `userinfo` and logout.
- More than one client.
- `plain` PKCE or flows without PKCE.

This is a test fixture: never expose it to a network you do not control.

## Development

Every check runs in Docker; nothing is installed on the host.

```bash
bin/check
```

`bin/check` runs what CI runs, in the same order:

| Step | Command |
|---|---|
| Formatting | `cargo fmt --all -- --check` |
| Lints | `cargo clippy --all-targets -- -D warnings` |
| Tests | `cargo nextest run --all-targets` |
| Advisories | `cargo deny check advisories sources` |
| Ignore review dates | `bin/check-review-dates deny.toml` |
| Image | `docker build` |

`tests/flow.rs` drives the whole flow with the `openidconnect` crate: discovery, PKCE, code exchange and ID-token verification.

| Workflow | Runs on | Does |
|---|---|---|
| `ci.yml` | Pull requests, and every `publish` run | cargo deny, fmt, clippy, tests, image build and smoke test |
| `publish.yml` | Pushes to `main` | Runs `ci.yml`, then pushes `latest` and `sha-<commit>` for amd64 and arm64 to Docker Hub |
| `pages.yml` | Pushes to `main` that touch `site/` | Publishes `site/` to GitHub Pages |
| Dependabot | Weekly | Cargo, GitHub Actions and Docker base image updates |

`bin/secure-repo` applies the GitHub settings once the repository is public: secret scanning with push protection, Dependabot alerts and fixes, private vulnerability reporting, approval for external contributors' workflow runs, required SHA pinning, the `dockerhub` environment limited to `main`, and a ruleset that requires a pull request with green checks on `main`.
