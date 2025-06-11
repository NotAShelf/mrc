{
  lib,
  rustPlatform,
}: let
  fs = lib.fileset;
  s = ../.;
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "mrc";
    version = "0.1.0";
    src = fs.toSource {
      root = s;
      fileset = fs.unions [
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /src))
        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

    cargoLock.lockFile = finalAttrs.src + /Cargo.lock;

    meta = {
      description = "IPC wrapper & command-line controller for MPV, the video player ";
      mainProgram = "mrc";
      license = lib.licenses.mpl20;
      maintainers = [lib.maintainers.NotAShelf];
    };
  })
