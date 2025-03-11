{ rustPlatform, ... }:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  version = cargoToml.package.version;
  pname = cargoToml.package.name;
in
rustPlatform.buildRustPackage {
  inherit pname version;
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
}
