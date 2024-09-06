{
  inputs = {
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:joelkoen/nixpkgs";
  };

  outputs = { nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        craneLib = crane.mkLib pkgs;
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

        devShells.default = with pkgs; mkShell {
          buildInputs = [
            sqlx-cli
          ];
        };
      }
    );
}
