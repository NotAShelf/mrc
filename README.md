# mrc

[mpv video player]: https://mpv.io/

Fast, robust and experimental JSON IPC wrapper for the [mpv video player]; comes
with a wrapper library, a CLI and a server for remote control.

## Hacking

Both CLI and server utilities require mpv to be started with the IPC socket
enabled.

```bash
mpv --idle --input-ipc-server=/tmp/mpvsocket
```

mrc does not handle TLS certificates for you. To run the remote control server,
you must create your own PKCS#12 certificates using OpenSSL.

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365
openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem
```

mrc then read the `TLS_PFX_PATH` and `TLS_PASSWORD` environment variables to
load them. You should also set `AUTH_TOKEN` for making authenticated requests to
the server remotely. Running without an auth token is "supported", but the
server will not fully function.

```bash
export TLS_PFX_PATH=/path/to/identity.pfx
export TLS_PASSWORD="your_identity_passphrase"
export AUTH_TOKEN="your_auth_token"
```

Then start the server with `cargo run --bin=server`.

How you handle environment is up to you, Systemd makes it somewhat easy if
running mrc as a service.

### Testing mpv IPC socket

If you suspect some IPC commands are not being sent properly, try sending them
manually first with something like socat.

```bash
echo '{ "command": ["pause"] }' | socat - /tmp/mpvsocket
```
