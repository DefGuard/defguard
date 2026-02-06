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

    # let git manage submodules
    self.submodules = true;
    proto = {
      url = "path:proto";
      flake = false;
    };
    defguard-ui = {
      url = "path:web/src/shared/defguard-ui";
      flake = false;
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-analyzer" "rust-src" "rustfmt" "clippy"];
      };
      # define shared build inputs
      nativeBuildInputs = with pkgs; [rustToolchain pkg-config];
      buildInputs = with pkgs; [openssl protobuf curl nodejs_24 pnpm];
    in {
      devShells.default = pkgs.mkShell {
        inherit nativeBuildInputs buildInputs;

        packages = with pkgs; [
          sqlx-cli
          just
          # TS/JS LSP
          vtsls
          # protobuf formatter
          buf
          # e2e
          playwright
          # release assets verification
          cosign
          # vulnerability scanner
          trivy
        ];

        # Specify the rust-src path (many editors rely on this)
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = true;
      };
    });
}
