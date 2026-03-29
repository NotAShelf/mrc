{
  mkShell,
  rust-analyzer,
  rustfmt,
  clippy,
  cargo,
  taplo,
  openssl,
  pkg-config,
  rustc,
}:
mkShell {
  name = "mpvrc";
  packages = [
    cargo
    rustc

    rust-analyzer
    clippy
    (rustfmt.override {asNightly = true;})
    taplo

    # For TLS and friends
    openssl
    pkg-config
  ];
}
