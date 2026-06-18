# dev-forge ⚒️

[![Test](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml/badge.svg)](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/ktakada42/dev-forge/graph/badge.svg)](https://codecov.io/gh/ktakada42/dev-forge)

A developer's workshop for everyday transformations — an interactive CLI REPL for common encoding, decoding, and conversion tasks.

## Installation

### Shell script (macOS / Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/ktakada42/dev-forge/main/install.sh | sh
```

Installs to `~/.local/bin` by default. Override with `INSTALL_DIR`:

```sh
curl -fsSL https://raw.githubusercontent.com/ktakada42/dev-forge/main/install.sh | INSTALL_DIR=/usr/local/bin sh
```

### Homebrew (macOS)

```sh
brew install ktakada42/tap/forge
```

To upgrade:

```sh
brew upgrade forge
```

### cargo install

Requires [Rust](https://rustup.rs) toolchain.

```sh
cargo install --git https://github.com/ktakada42/dev-forge
```

### Build from source

```sh
git clone https://github.com/ktakada42/dev-forge
cd dev-forge
cargo build --release
./target/release/forge
```

## Usage

Start the REPL:

```sh
forge
```

### Navigation

| Command | Description |
|---|---|
| `/timestamp` | Switch to timestamp tool |
| `/base64` | Switch to Base64 tool (auto-detect encode/decode) |
| `/base64-encode` | Switch to Base64 encode |
| `/base64-decode` | Switch to Base64 decode |
| `/url` | Switch to URL tool (auto-detect encode/decode) |
| `/url-encode` | Switch to URL encode |
| `/url-decode` | Switch to URL decode |
| `/jwt` | Switch to JWT tool |
| `/help` | Show help for the current tool |
| `exit` / `quit` | Exit dev-forge |

### Timestamp

Convert between Unix timestamps and human-readable datetime strings.

```
forge> /timestamp
forge(timestamp)> 1749812345
2025-06-13 15:19:05 JST

forge(timestamp)> 1749812345 UTC
2025-06-13 06:19:05 UTC

forge(timestamp)> 2025-06-13 15:19:05 Asia/Tokyo
1749812345
```

**Supported datetime formats:**
- `2025-06-13T15:19:05+09:00`
- `2025-06-13 15:19:05`
- `2025/06/13 15:19:05`

**Timezone examples:** `Asia/Tokyo`, `UTC`, `America/New_York`, `Europe/London`, `+09:00`

Millisecond timestamps are auto-detected.

### Base64

`/base64` auto-detects whether to encode or decode based on the input value.

```
forge> /base64
forge(base64)> hello
[auto: encode]
aGVsbG8=

forge(base64)> aGVsbG8=
[auto: decode]
hello
```

You can also pass the value inline from the root prompt:

```
forge> /base64 aGVsbG8=
[auto: decode]
hello
```

To force a specific operation, use `/base64-encode` or `/base64-decode`:

```
forge> /base64-encode
forge(base64-encode)> hello
aGVsbG8=

forge> /base64-decode
forge(base64-decode)> aGVsbG8=
hello
```

**Note on auto-detection:** `/base64` infers decode by attempting to base64-decode the input and checking whether the result is valid UTF-8 text. This works well for the common case of encoding and decoding text. However, if the base64 input encodes binary data (e.g. images, encrypted bytes), the decoded bytes are likely not valid UTF-8, and auto-detection will incorrectly treat the input as plain text to encode. Use `/base64-decode` explicitly in those cases.

### URL

`/url` auto-detects whether to encode or decode. Decode is chosen when the input contains at least one percent-encoded sequence (`%XX`).

```
forge> /url
forge(url)> hello world
[auto: encode]
hello%20world

forge(url)> hello%20world
[auto: decode]
hello world
```

To force a specific operation, use `/url-encode` or `/url-decode`:

```
forge> /url-encode
forge(url-encode)> hello world
hello%20world

forge> /url-decode
forge(url-decode)> hello%20world
hello world
```

### JWT

Decode JWT header and payload (no signature verification).

```
forge> /jwt
forge(jwt)> decode
Input:
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature
{
  "header": { "alg": "HS256", "typ": "JWT" },
  "payload": { "sub": "1234567890" }
}
```

## License

MIT
