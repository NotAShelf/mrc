{rustPlatform}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "mrc";
  version = "0.0.1";
  src = ./.;

  cargoLock.lockFile = finalAttrs.src + /Cargo.lock;
  meta = {
    description = "MPV IPC wrapper";
    mainProgram = "mrc";
  };
})
