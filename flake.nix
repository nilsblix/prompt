{
    description = "Prompt written in rust";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, flake-utils }:
        flake-utils.lib.eachDefaultSystem (system:
            let
                pkgs = import nixpkgs {
                    inherit system;
                };
            in {
                devShells.default = pkgs.mkShell {
                    packages = with pkgs; [
                        cargo
                        rustc
                    ];
                };

                packages.default = pkgs.rustPlatform.buildRustPackage {
                    pname = "prompt";
                    version = "0.1.0";
                    src = ./.;
                    cargoLock = {
                        lockFile = ./Cargo.lock;
                    };
                    nativeBuildInputs = with pkgs; [ cargo rustc ];
                };
            }
        );
}
