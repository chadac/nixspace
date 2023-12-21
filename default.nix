{ lib, stdenv, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "nixspace";
  version = "1.0.0";

  src = lib.cleanSource ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
}
