# dev-forge ⚒️

[![Test](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml/badge.svg)](https://github.com/ktakada42/dev-forge/actions/workflows/test.yml)
[![Release](https://github.com/ktakada42/dev-forge/actions/workflows/release.yml/badge.svg)](https://github.com/ktakada42/dev-forge/actions/workflows/release.yml)
[![codecov](https://codecov.io/gh/ktakada42/dev-forge/graph/badge.svg)](https://codecov.io/gh/ktakada42/dev-forge)
[![GitHub release](https://img.shields.io/github/v/release/ktakada42/dev-forge)](https://github.com/ktakada42/dev-forge/releases/latest)
[![License](https://img.shields.io/github/license/ktakada42/dev-forge)](LICENSE)

![dev-forge picking a tool and encoding a string](assets/demo.gif)

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

Set `FORGE_NO_ANIMATION=1` to skip the startup animation (a still frame is
printed instead), and `NO_COLOR=1` to drop the colors. Both are suppressed
automatically when output is not a terminal or when `TERM=dumb`.

### Navigation

dev-forge asks which tool you want with a list, not a command:

```
? Select a tool  (type to filter)
> timestamp  Unix timestamp <-> datetime conversion
  base64     Base64 encode/decode
  url        URL encode/decode
  jwt        JWT decode (no signature verification)
  up/down move   enter select   esc/ctrl-d quit
```

| Key | Description |
|---|---|
| up / down (or ctrl-p / ctrl-n, tab) | Move the cursor |
| any letter | Filter the list |
| enter | Select |
| esc, ctrl-d (or ctrl-c) | Back — from the tool list, quit |

Tools that go both ways ask the same way, so `base64` is followed by a list of
`encode` / `decode`. Once picked, the direction stays picked and every line you
type is converted:

```
? Select a tool  base64
? base64  encode
  Text to encode.
  esc, ctrl-c        back to the tool list
  ctrl-d             quit

forge(base64 encode)> hello
aGVsbG8=
forge(base64 encode)> hello world
aGVsbG8gd29ybGQ=
```

Nothing typed at that prompt is a command — a payload that reads like `exit` is
encoded, not obeyed. Esc and ctrl-c go back to the tool list, ctrl-d quits, and
Enter on an empty line does nothing. The keys mean the same thing at the prompt
as they do in the lists.

A payload can span lines: shift+enter starts another one instead of sending
what is there. Telling shift+enter from enter needs the kitty keyboard
protocol, which Ghostty, kitty, WezTerm and foot speak; without it the two keys
are the same byte and the hint names alt+enter instead, which arrives as
ESC+Enter (on macOS, terminals send that only with Option set to act as Meta).
Either way the hint names the key that works where you are running, and pasting
text that already has newlines in it works in any terminal. Tab types a tab,
and spaces at either end of a line are part of the payload — what you see on
the line is what gets converted.

### Timestamp

Convert between Unix timestamps and human-readable datetime strings.

```
forge(timestamp)> 1749812345
2025-06-13T19:59:05+09:00

forge(timestamp)> 1749812345 UTC
2025-06-13T10:59:05+00:00

forge(timestamp)> 2025-06-13 15:19:05 Asia/Tokyo
1749795545
```

**Supported datetime formats:**
- `2025-06-13T15:19:05+09:00`
- `2025-06-13 15:19:05`
- `2025/06/13 15:19:05`

**Timezone examples:** `Asia/Tokyo`, `UTC`, `America/New_York`, `Europe/London`, `+09:00`

Millisecond timestamps are auto-detected.

### Base64

```
forge(base64 encode)> hello
aGVsbG8=

forge(base64 decode)> aGVsbG8=
hello
```

### URL

```
forge(url encode)> hello world
hello%20world

forge(url decode)> hello%20world
hello world
```

### JWT

Decode JWT header and payload (no signature verification).

```
forge(jwt decode)> eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature
Header
{
  "alg": "HS256",
  "typ": "JWT"
}

Payload
{
  "sub": "1234567890"
}
```

## Command mode

The same tools are subcommands, for scripts and pipes. The names match the ones
in the lists, so what you learn interactively is what you type here:

```sh
forge base64 encode hello
echo hello | forge base64 encode
forge url decode "hello%20world"
forge jwt decode "$TOKEN"
forge timestamp 1749812345 --tz UTC
```

A value can be given as an argument or piped on stdin. Interactive mode needs a
terminal, so `forge` with no subcommand in a pipe points you here instead.

## License

MIT
