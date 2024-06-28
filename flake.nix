{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        craneLib = crane.lib.${system};
      in
      {
        packages.default = craneLib.buildPackage {
          src = lib.cleanSourceWith {
            src = craneLib.path ./.;
            filter = path: type:
              lib.hasInfix "/.sqlx" path
              || lib.hasSuffix ".sql" path
              || craneLib.filterCargoSources path type;
          };
        };
      });
}
