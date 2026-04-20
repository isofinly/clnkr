{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
            pkg-config
            cargo-deny
            sqlx-cli
            postgresql
            bun
            openssl
            llvm
            mecab
          ];

          env = {
            OUT_DIR = "~/.cargo-target/proto";
            DATABASE_URL = "postgresql://postgres:postgres@localhost:5432";
          };

          shellHook = ''
            rustc --version
            bun --version
          '';
        };
      }
    );
}
