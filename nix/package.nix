{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "mrc";
  version = "0.1.0";
  src = builtins.path {
    path = ../.;
    name = "mrc";
  };

  cargoLock.lockFile = finalAttrs.src + /Cargo.lock;

  meta = {
    description = "IPC wrapper & command-line controller for MPV, the video player ";
    mainProgram = "mrc";
    license = lib.licenses.mpl20;
  };
})
