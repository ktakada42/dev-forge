# timestamp

Convert between Unix timestamps and human-readable datetime strings. The
direction is decided by the input: an integer is a timestamp to expand, and
anything else is a datetime to reduce.

```sh
forge timestamp [--tz <TZ>] [VALUE]
```

In interactive mode the tool is never asked which way to go, so there is no
`encode` / `decode` list — pick `timestamp` and start pasting.

## Timestamp to datetime

Output is RFC 3339, in your local timezone unless a timezone is given:

```
forge(timestamp)> 1749812345
2025-06-13T19:59:05+09:00
```

```sh
forge timestamp 1749812345 --tz UTC
2025-06-13T10:59:05+00:00
```

Millisecond timestamps are auto-detected — a value of `1_000_000_000_000` or
more is read as milliseconds, and the fraction is kept:

```sh
forge timestamp 1749812345678 --tz UTC
2025-06-13T10:59:05.678+00:00
```

That threshold is 2001-09-09 in seconds, so any second-based timestamp from
this century stays a second-based timestamp. Negative values (before 1970)
are always read as seconds.

## Datetime to timestamp

```sh
forge timestamp "2025-06-13 15:19:05" --tz Asia/Tokyo
1749795545
```

Supported input formats:

| Format | Example |
| --- | --- |
| RFC 3339 | `2025-06-13T15:19:05+09:00` |
| Space-separated | `2025-06-13 15:19:05` |
| Slash-separated | `2025/06/13 15:19:05` |

RFC 3339 carries its own offset, so `--tz` is ignored for that form. The other
two have no timezone of their own: they are read in `--tz` if given, and in
your local timezone otherwise.

## Timezones

`--tz` accepts an IANA name or a fixed UTC offset:

```
Asia/Tokyo   UTC   America/New_York   Europe/London   +09:00   -05:00
```

An offset must be written in full, sign and minutes included (`+09:00`, not
`+9` or `+0900`).

In interactive mode there is no flag to pass, so the timezone goes on the line
after the value:

```
forge(timestamp)> 1749812345 UTC
2025-06-13T10:59:05+00:00

forge(timestamp)> 2025-06-13 15:19:05 Asia/Tokyo
1749795545
```

The last word is read as the timezone whenever the line has three or more
words, or two words whose first one is an integer. `2025-06-13 15:19:05` is
therefore a datetime in your local timezone, not a date with a timezone of
`15:19:05`.

## Errors

| Message | Cause |
| --- | --- |
| `Unknown timezone: '...'` | Not an IANA name and not a `+HH:MM` offset |
| `Cannot parse: ...` | The value is neither an integer nor one of the formats above |
| `Invalid timestamp` | The integer is outside the range of representable dates |
| `Ambiguous datetime in specified timezone` | The wall-clock time does not exist, or exists twice, because of a DST transition |

---

See also: [interactive mode](../interactive-mode.md) · [command mode](../command-mode.md)
