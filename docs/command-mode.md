# Command mode

The same tools are subcommands, for scripts and pipes. The names match the ones
in the interactive lists, so what you learn there is what you type here.

```sh
forge base64 encode hello
forge url decode "hello%20world"
forge jwt decode "$TOKEN"
forge timestamp 1749812345 --tz UTC
```

## Synopsis

```
forge [COMMAND]

Commands:
  timestamp  Unix timestamp <-> datetime conversion
  base64     Base64 encode/decode
  url        URL encode/decode
  jwt        JWT decode (no signature verification)
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

With no subcommand, `forge` starts [interactive mode](interactive-mode.md).

| Command | Arguments |
| --- | --- |
| `forge timestamp [--tz <TZ>] [VALUE]` | [details](tools/timestamp.md) |
| `forge base64 encode\|decode [VALUE]` | [details](tools/base64.md) |
| `forge url encode\|decode [VALUE]` | [details](tools/url.md) |
| `forge jwt decode [VALUE]` | [details](tools/jwt.md) |

## Input

Every tool takes its value as an argument or on stdin, so both of these work:

```sh
forge base64 encode hello
echo hello | forge base64 encode
```

Trailing newlines are stripped from piped input — `echo` adds one and it is
almost never part of the payload. Everything else arrives as sent, including
interior newlines and leading or trailing spaces, so quote arguments that carry
them:

```sh
forge base64 encode " a "        # encodes the spaces too
printf 'two\nlines' | forge base64 encode
```

With no argument and no pipe, dev-forge has nothing to work on and says so:

```sh
$ forge base64 encode
Error: No input provided. Pass a value as argument or pipe via stdin.
```

## Output and exit codes

The result goes to stdout with a trailing newline, so the output composes:

```sh
forge base64 encode hello | forge base64 decode
```

Errors go to stderr, prefixed with `Error:`.

| Status | Meaning |
| --- | --- |
| `0` | Converted; the result is on stdout |
| `1` | No input, an unparseable value, or interactive mode asked for without a terminal |
| `2` | Unknown subcommand or bad flag (reported by the argument parser) |

Because errors never land on stdout, `set -e` and a plain assignment are enough
to use dev-forge from a script:

```sh
ts=$(forge timestamp "2025-06-13 15:19:05" --tz Asia/Tokyo) || exit 1
```
