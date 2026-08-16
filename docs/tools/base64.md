# base64

Base64 encode and decode, using the standard alphabet with padding
([RFC 4648 §4](https://datatracker.ietf.org/doc/html/rfc4648#section-4)).

```sh
forge base64 encode [VALUE]
forge base64 decode [VALUE]
```

In interactive mode, picking `base64` asks for the direction next; the answer
sticks until you go back.

## Encode

The input is treated as UTF-8 text and encoded byte for byte — nothing is
trimmed, so spaces and newlines inside the payload survive the round trip.

```
forge(base64 encode)> hello
aGVsbG8=

forge(base64 encode)> あ
44GC
```

## Decode

Whitespace around the input is trimmed first, so a token copied out of a header
or a log line decodes without cleanup.

```
forge(base64 decode)> aGVsbG8=
hello
```

Decoding is the stricter direction, and it fails in two ways:

| Message | Cause |
| --- | --- |
| `Decode error: ...` | Not valid standard Base64 — an out-of-alphabet character, or a length that cannot be padding-correct |
| `UTF-8 error: ...` | The bytes decoded fine but are not text |

```
forge(base64 decode)> not-base64!
Error: Decode error: Invalid symbol 45, offset 3.
```

Because the output is a Rust `String`, binary payloads cannot be printed — a
Base64-encoded PNG decodes to bytes that are not UTF-8 and reports the second
error above. dev-forge is a text tool; use `base64 -d` for binaries.

URL-safe Base64 (`-` and `_`, usually unpadded) is not accepted here. It is
what JWT parts are encoded with, and [`jwt`](jwt.md) handles those.

## Round trip

```sh
forge base64 encode hello | forge base64 decode
```

---

See also: [interactive mode](../interactive-mode.md) · [command mode](../command-mode.md)
