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
    openssl
    pkg-config
    rustc
  ];
}
