{
  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = with pkgs; mkShell rec {
          nativeBuildInputs = [
            pkg-config
          ];

          buildInputs = [
            rust-bin.stable."1.83.0".default
            rust-analyzer
            udev alsa-lib vulkan-loader
            xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr
            libxkbcommon wayland
            systemd
          ];

          LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
        };
      }
    );
}

