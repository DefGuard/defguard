{
  description = "Rust development flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml {
        extensions = ["rust-analyzer" "rust-src" "rustfmt" "clippy"];
      };
      # this is how we can tell crane to use our toolchain!
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
      # filter out unnecessary files from source
      # sqlxOrProtoFilter = path: _type: (builtins.match ".*sql$" path != null) || (builtins.match ".*proto$" path != null);
      sqlOrProtoFilter = path:
        pkgs.lib.any (suffix: pkgs.lib.hasSuffix suffix path) [
          # Keep SQL files
          ".sql"
          # Keep protobuf files
          ".proto"
        ];
      # protoFilter = path: _type: builtins.match ".*proto$" path != null;
      # sqlFilter = path: _type: builtins.match ".*sql$" path != null;
      srcFilter = path: type: (sqlOrProtoFilter path) || (craneLib.filterCargoSources path type);
      src = pkgs.lib.cleanSourceWith {
        src = ./.; # The original, unfiltered source
        filter = srcFilter;
        name = "source"; # Be reproducible, regardless of the directory name
      };
      # define shared build inputs
      nativeBuildInputs = with pkgs; [rustToolchain pkg-config];
      buildInputs = with pkgs; [openssl protobuf curl];
      # because we'll use it for both `cargoArtifacts` and `bin`
      commonArgs = {
        inherit src buildInputs nativeBuildInputs;
      };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      # remember, `set1 // set2` does a shallow merge:
      bin = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          # don't run tests when building
          doCheck = false;

          SQLX_OFFLINE = 1;
        });
    in {
      packages = {
        # that way we can build `bin` specifically,
        # but it's also the default.
        inherit bin;
        default = bin;
      };
      devShells.default = pkgs.mkShell {
        # instead of passing `buildInputs` / `nativeBuildInputs`,
        # we refer to an existing derivation here
        inputsFrom = [bin];

        packages = with pkgs; [
          sqlx-cli
        ];

        # Specify the rust-src path (many editors rely on this)
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      };
    });
}
