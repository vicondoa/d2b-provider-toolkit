{
  description = "Canonical d2b provider SDK distribution";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    d2b = {
      url = "github:vicondoa/d2b/4018d9c9652bd826c2e6a9abccdcdcafb832d944";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      d2b,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          canonicalSource = pkgs.runCommand "d2b-provider-canonical-source" { } ''
            mkdir -p "$out"
            cp -R ${d2b}/. "$out/"
            chmod -R u+w "$out"
            cp -R ${self}/contract/docs/. "$out/docs/"
          '';
          distributionSource = pkgs.runCommand "d2b-provider-toolkit-source-tree" { } ''
            mkdir -p "$out"
            cp -R ${self}/. "$out/"
            chmod -R u+w "$out"
            rm -rf "$out/canonical/d2b"
            mkdir -p "$out/canonical/d2b"
            cp -R ${canonicalSource}/. "$out/canonical/d2b/"
          '';
          toolkit = pkgs.rustPlatform.buildRustPackage {
            pname = "d2b-provider-toolkit";
            version = "0.1.0";
            src = distributionSource;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "--workspace"
              "--all-targets"
            ];
            cargoTestFlags = [
              "--workspace"
              "--all-targets"
            ];
            nativeCheckInputs = [
              pkgs.git
              pkgs.jq
            ];
            postCheck = ''
              bash scripts/check-distribution-policy.sh
              cargo run --offline --quiet -p d2b-provider-source -- verify
              cargo run --offline --quiet -p d2b-provider-conformance -- self-test
              cargo run --offline --quiet -p d2b-provider-azure-fake
            '';
            postInstall = ''
              mkdir -p "$out/share/d2b-provider-toolkit"
              cp -R docs templates contract pins \
                "$out/share/d2b-provider-toolkit/"
            '';
          };
          sourceArchive = pkgs.runCommand "d2b-provider-toolkit-0.1.0-source" {
            nativeBuildInputs = [
              pkgs.gnutar
              pkgs.gzip
            ];
          } ''
            mkdir -p "$out" archive/d2b-provider-toolkit-0.1.0
            cp -R ${distributionSource}/. archive/d2b-provider-toolkit-0.1.0/
            chmod -R u+w archive
            tar \
              --sort=name \
              --mtime="@1" \
              --owner=0 \
              --group=0 \
              --numeric-owner \
              -C archive \
              -czf "$out/d2b-provider-toolkit-0.1.0-source.tar.gz" \
              d2b-provider-toolkit-0.1.0
          '';
          contractDocs = pkgs.runCommand "d2b-provider-contract-docs" { } ''
            mkdir -p "$out/share/d2b-provider-toolkit"
            cp -R ${self}/contract/docs "$out/share/d2b-provider-toolkit/"
          '';
          releasePackagingCheck = pkgs.runCommand "d2b-provider-release-packaging-check" {
            nativeBuildInputs = [
              pkgs.bash
              pkgs.binutils
              pkgs.coreutils
              pkgs.gawk
              pkgs.gnugrep
              pkgs.gnutar
              pkgs.gzip
            ];
          } ''
            for binary in ${toolkit}/bin/*; do
              interpreter="$(
                readelf --program-headers "$binary" |
                  sed -n 's/.*Requesting program interpreter: \(.*\)]/\1/p'
              )"
              case "$interpreter" in
                /nix/store/*) ;;
                *) exit 1 ;;
              esac
              readelf --dynamic "$binary" | grep -F '(NEEDED)' >/dev/null
              readelf --dynamic "$binary" |
                grep -E '\((RUNPATH|RPATH)\).*/nix/store/' >/dev/null
            done
            touch "$out"
          '';
        in
        {
          default = toolkit;
          d2b-provider-toolkit = toolkit;
          sourceArchive = sourceArchive;
          contractDocs = contractDocs;
          releasePackagingCheck = releasePackagingCheck;
        }
      );

      checks = forAllSystems (
        system:
        let
          packages = self.packages.${system};
        in
        {
          rust = packages.d2b-provider-toolkit;
          source-archive = packages.sourceArchive;
          contract-docs = packages.contractDocs;
          release-packaging = packages.releasePackagingCheck;
        }
      );

      apps = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolkit = self.packages.${system}.d2b-provider-toolkit;
          conformance = pkgs.writeShellApplication {
            name = "d2b-provider-conformance";
            text = ''
              exec ${toolkit}/bin/d2b-provider-conformance self-test "$@"
            '';
          };
          sourceCheck = pkgs.writeShellApplication {
            name = "d2b-provider-source-check";
            text = ''
              exec ${toolkit}/bin/d2b-provider-source verify "$@"
            '';
          };
        in
        {
          default = {
            type = "app";
            program = "${conformance}/bin/d2b-provider-conformance";
          };
          conformance = {
            type = "app";
            program = "${conformance}/bin/d2b-provider-conformance";
          };
          source-check = {
            type = "app";
            program = "${sourceCheck}/bin/d2b-provider-source-check";
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              git
              jq
              rustc
              rustfmt
            ];
          };
        }
      );
    };
}
