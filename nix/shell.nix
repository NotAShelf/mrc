{
  mkShell,
  rust-analyzer,
  rustfmt,
  clippy,
  cargo,
  gcc,
  openssl,
  pkg-config,
  rustc,
}:
mkShell {
  name = "mrc";
  packages = [
    rust-analyzer
    rustfmt
    clippy
    cargo
    gcc
    clippy
    rustfmt
    rustc

    # For TLS and friends
    openssl
    pkg-config
  ];
}
