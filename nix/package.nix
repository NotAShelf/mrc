{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
}: let
  fs = lib.fileset;
  s = ../.;
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "mrc";
    version = (builtins.fromTOML (builtins.readFile (s + /Cargo.toml))).package.version;

    src = fs.toSource {
      root = s;
      fileset = fs.unions [
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /src))
        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

    strictDeps = true;
    enableParallelBuilding = true;

    nativeBuildInputs = [
      pkg-config
    ];

    buildInputs = [
      openssl
    ];

    cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
    useFetchCargoVendor = true;

    meta = {
      description = "IPC wrapper & command-line controller for MPV, the video player";
      homePage = "https://github.com/notashelf/mrc";
      mainProgram = "mrc";
      license = lib.licenses.mpl20;
      maintainers = [lib.maintainers.NotAShelf];
    };
  })
