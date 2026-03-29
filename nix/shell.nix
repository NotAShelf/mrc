{
  mkShell,
  rust-analyzer,
  rustfmt,
  clippy,
  cargo,
  openssl,
  pkg-config,
  rustc,
}:
mkShell {
  name = "mrc";
  packages = [
    cargo
    rustc

    rust-analyzer
    clippy
    (rustfmt.override {asNightly = true;})

    # For TLS and friends
    openssl
    pkg-config
  ];
}
