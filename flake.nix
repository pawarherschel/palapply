{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devenv.url = "github:cachix/devenv";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, devenv, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ devenv.flakeModule ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        {
          inputs',
          pkgs,
          ...
        }:
        {

          devenv = {
            shells.default = {
              cachix.enable = true;
              cachix.pull = [ "fenix" ];
              languages.rust = {
                enable = true;
                toolchain = inputs'.fenix.packages.stable.completeToolchain;
                mold.enable = pkgs.stdenv.isLinux;
              };

              packages = with pkgs; [
                pkg-config
                jetbrains.rust-rover
              ];

              env = {
                RUST_BACKTRACE = "1";
              };
            };
          };
        };
    };
}
