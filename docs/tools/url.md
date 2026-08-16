# url

URL percent-encoding, the strict kind: everything outside the unreserved set
`A-Z a-z 0-9 - _ . ~` is encoded.

```sh
forge url encode [VALUE]
forge url decode [VALUE]
```

In interactive mode, picking `url` asks for the direction next; the answer
sticks until you go back.

## Encode

```
forge(url encode)> hello world
hello%20world

forge(url encode)> a=1&b=2
a%3D1%26b%3D2
```

This is component encoding — it is what you want for a query-string value or a
path segment, and it encodes a whole URL into an unusable one:

```sh
forge url encode 'https://example.com/a b?x=1'
https%3A%2F%2Fexample.com%2Fa%20b%3Fx%3D1
```

Encode the parts, not the URL. Two more consequences worth knowing:

- A space becomes `%20`, never `+`. The `+` convention belongs to
  `application/x-www-form-urlencoded`, not to URLs.
- `%` is itself encoded, so encoding twice compounds: `hello%20world` becomes
  `hello%2520world`.

Non-ASCII is encoded as UTF-8 bytes:

```sh
forge url encode 'あ'
%E3%81%82
```

## Decode

Whitespace around the input is trimmed first.

```
forge(url decode)> hello%20world
hello world

forge(url decode)> %E3%81%82
あ
```

Decoding is lenient about anything that is not a valid triplet — `%zz` and a
bare `%` are passed through unchanged rather than rejected — and `+` stays a
`+`, matching the encode side. Only bytes that do not form valid UTF-8 are an
error:

```
forge(url decode)> %FF
Error: Decode error: invalid utf-8 sequence of 1 bytes from index 0
```

## Round trip

```sh
forge url encode 'a=1&b=2' | forge url decode
```

---

See also: [interactive mode](../interactive-mode.md) · [command mode](../command-mode.md)
