{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        craneLib = crane.lib.${system};
        buildPackage = name: craneLib.buildPackage {
          pname = name;
          version = "0.1.0";

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          strictDeps = true;
          cargoExtraArgs = "-p " + name;

          buildInputs = [
            # Add additional build inputs here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
        };

        tcp-throughput-c2s-client = buildPackage "tcp_throughput_c2s_client";
        tcp-throughput-s2c-client = buildPackage "tcp_throughput_s2c_client";
        udp-throughput-c2s-client = buildPackage "udp_throughput_c2s_client";
      in
      {
        checks = {
          inherit tcp-throughput-c2s-client;
        };

        packages.tcp-throughput-c2s-client = tcp-throughput-c2s-client;
        packages.tcp-throughput-s2c-client = tcp-throughput-s2c-client;
        packages.udp-throughput-c2s-client = udp-throughput-c2s-client;
        apps.tcp-throughput-c2s-client = flake-utils.lib.mkApp {
          drv = tcp-throughput-c2s-client;
        };
        apps.tcp-throughput-s2c-client = flake-utils.lib.mkApp {
          drv = tcp-throughput-s2c-client;
        };
        apps.udp-throughput-c2s-client = flake-utils.lib.mkApp {
          drv = udp-throughput-c2s-client;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
          ];
        };
      });
}
