Creating a PKCS#12 certificate file using OpenSSL:

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365
openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem
```

Before running the server:

```bash
export TLS_PFX_PATH=/path/to/identity.pfx
export TLS_PASSWORD="your_identity_passphrase"
export AUTH_TOKEN="your_auth_token"
```

How you handle environment is up to you, Systemd makes it somewhat easy.
