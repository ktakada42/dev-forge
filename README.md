# dev-forge ⚒️

[![Test](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml/badge.svg)](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml)
[![Release](https://github.com/ktakada42/dev-forge/actions/workflows/release.yml/badge.svg)](https://github.com/ktakada42/dev-forge/actions/workflows/release.yml)
[![codecov](https://codecov.io/gh/ktakada42/dev-forge/graph/badge.svg)](https://codecov.io/gh/ktakada42/dev-forge)
[![crates.io](https://img.shields.io/crates/v/dev-forge)](https://crates.io/crates/dev-forge)
[![GitHub release](https://img.shields.io/github/v/release/ktakada42/dev-forge)](https://github.com/ktakada42/dev-forge/releases/latest)
[![License](https://img.shields.io/github/license/ktakada42/dev-forge)](LICENSE)

![dev-forge picking a tool and encoding a string](assets/demo.gif)

A developer's workshop for everyday transformations — an interactive CLI REPL
for common encoding, decoding, and conversion tasks.

Pick a tool from a list instead of remembering a command, then paste payload
after payload. Nothing typed at the prompt is a command, so a payload that
reads like `exit` gets converted rather than obeyed. Every tool is a plain
subcommand too, so the names you learn interactively are the ones you type in
scripts and pipes.

## Installation

### Shell script (macOS / Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/ktakada42/dev-forge/main/install.sh | sh
```

Installs to `~/.local/bin` by default. Override with `INSTALL_DIR`:

```sh
curl -fsSL https://raw.githubusercontent.com/ktakada42/dev-forge/main/install.sh | INSTALL_DIR=/usr/local/bin sh
```

### Homebrew (macOS / Linux)

```sh
brew install ktakada42/tap/forge
brew upgrade forge
```

### cargo install

Requires the [Rust](https://rustup.rs) toolchain. The crate is `dev-forge`; the
binary it installs is `forge`.

```sh
cargo install dev-forge
```

### Build from source

```sh
git clone https://github.com/ktakada42/dev-forge
cd dev-forge
cargo build --release
./target/release/forge
```

## Tools

| Tool | What it does | Details |
| --- | --- | --- |
| `timestamp` | Unix timestamp ↔ datetime, in any IANA timezone or UTC offset | [docs](docs/tools/timestamp.md) |
| `base64` | Base64 encode / decode | [docs](docs/tools/base64.md) |
| `url` | URL percent-encoding encode / decode | [docs](docs/tools/url.md) |
| `jwt` | Decode a JWT header and payload (no signature verification) | [docs](docs/tools/jwt.md) |

Two ways to reach them:

```sh
forge                              # interactive: pick a tool, then paste payloads
forge base64 encode hello          # one-shot subcommand
echo hello | forge base64 encode   # or from stdin
```

## Documentation

- [Interactive mode](docs/interactive-mode.md) — the tool picker, key bindings,
  multi-line payloads, and the environment variables that control animation and
  color.
- [Command mode](docs/command-mode.md) — subcommands, stdin, and exit codes for
  scripts and pipes.
- [Contributing](CONTRIBUTING.md) — how to build, test, and open a pull request.

## License

MIT — see [LICENSE](LICENSE).
