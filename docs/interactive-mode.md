# Interactive mode

Start the REPL with no subcommand:

```sh
forge
```

Interactive mode needs a terminal on both ends — stdin for the keys, stdout for
the frames. Running `forge` with no subcommand in a pipe prints a pointer to
[command mode](command-mode.md) and exits with status `1`.

## Picking a tool

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
| --- | --- |
| up / down (or ctrl-p / ctrl-n, tab / shift-tab) | Move the cursor |
| any letter | Filter the list |
| backspace (or ctrl-h) | Delete a character from the filter |
| enter | Select the highlighted row |
| esc, ctrl-c, ctrl-d (or ctrl-g) | Back — from the tool list, quit |

Tools that go both ways ask the same way, so `base64` is followed by a list of
`encode` / `decode`. `jwt` only decodes and `timestamp` decides from the input,
so neither is asked.

## Converting

Once picked, the direction stays picked and every line you type is converted:

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
encoded, not obeyed. Escape and ctrl-c go back to the tool list, ctrl-d quits,
and Enter on an empty line does nothing. The keys mean the same thing at the
prompt as they do in the lists.

Errors are printed and the prompt stays where it is, so a bad paste costs you
the line and nothing more:

```
forge(base64 decode)> not-base64!
Error: Decode error: Invalid symbol 45, offset 3.
```

## Multi-line payloads

A payload can span lines: shift+enter starts another one instead of sending
what is there. Telling shift+enter from enter needs the kitty keyboard
protocol, which Ghostty, kitty, WezTerm and foot speak; without it the two keys
are the same byte and the hint names alt+enter instead, which arrives as
ESC+Enter (on macOS, terminals send that only with Option set to act as Meta).

Either way the hint names the key that works where you are running, and pasting
text that already has newlines in it works in any terminal.

Tab types a tab, and spaces at either end of a line are part of the payload —
what you see on the line is what gets converted.

## Environment variables

| Variable | Effect |
| --- | --- |
| `FORGE_NO_ANIMATION` | Skip the startup animation; a still frame is printed instead |
| `NO_COLOR` | Drop the colors ([no-color.org](https://no-color.org)) |
| `TERM=dumb` | Skip the animation, drop the colors, and rule out the picker |

Both the animation and the colors are suppressed automatically when output is
not a terminal, so nothing has to be set to make a pipe behave.

```sh
FORGE_NO_ANIMATION=1 NO_COLOR=1 forge
```

## Per-tool details

- [timestamp](tools/timestamp.md)
- [base64](tools/base64.md)
- [url](tools/url.md)
- [jwt](tools/jwt.md)
