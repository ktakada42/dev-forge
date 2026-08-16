# jwt

Decode the header and payload of a JSON Web Token and pretty-print both.

```sh
forge jwt decode [VALUE]
```

There is only one direction, so interactive mode does not ask: pick `jwt` and
paste tokens.

> [!IMPORTANT]
> **The signature is never verified.** This tool tells you what a token
> *claims*, not whether the claim is true. Anyone can mint a token that says
> `"admin": true`. Never make an authorization decision from this output —
> verify with the issuer's key, in your application, using a JWT library.

## Output

```
forge(jwt decode)> eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
Header
{
  "alg": "HS256",
  "typ": "JWT"
}

Payload
{
  "name": "John Doe",
  "sub": "1234567890"
}
```

Both parts are Base64url-decoded (`-` and `_`, no padding) and parsed as JSON,
then printed with two-space indentation. Object keys come out sorted, so the
same token always prints the same way and two tokens diff cleanly.

The third part is the signature. It is not decoded, not shown, and not checked,
so a two-part token — header and payload with no signature — decodes fine:

```sh
forge jwt decode "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0"
```

Timestamp claims (`exp`, `iat`, `nbf`) are printed as the numbers they are.
Feed one to [`timestamp`](timestamp.md) to read it:

```sh
forge timestamp 1516239022 --tz UTC
```

## Errors

| Message | Cause |
| --- | --- |
| `Invalid JWT: expected at least 2 parts separated by '.'` | No `.` in the input |
| `Header decode failed: ...` / `Payload decode failed: ...` | The part is not Base64url, or not UTF-8 once decoded |
| `Header JSON error: ...` / `Payload JSON error: ...` | The part decoded to text that is not JSON |

An encrypted token (JWE, five parts) carries an encrypted key where a signed
token carries the payload, so reading one fails at the second part. Only signed
tokens (JWS) can be read this way.

---

See also: [interactive mode](../interactive-mode.md) · [command mode](../command-mode.md)
