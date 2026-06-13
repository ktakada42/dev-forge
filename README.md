# dev-forge ⚒️

A developer's workshop for everyday transformations — an interactive CLI REPL for common encoding, decoding, and conversion tasks.

## Installation

### Homebrew (macOS)

Coming soon.

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
| `/base64` | Switch to Base64 tool |
| `/url` | Switch to URL tool |
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

```
forge> /base64
forge(base64)> encode
Input:
hello
aGVsbG8=

forge(base64)> decode
Input:
aGVsbG8=
hello
```

### URL

```
forge> /url
forge(url)> encode
Input:
hello world
hello%20world

forge(url)> decode
Input:
hello%20world
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
