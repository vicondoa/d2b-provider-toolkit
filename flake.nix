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
          # Hermetic format/lint gates. These reuse the same distributionSource
          # tree and pinned rustPlatform toolchain as the `toolkit` package
          # (no ambient `canonical/d2b` submodule checkout, no network) so
          # `nix flake check` covers `cargo fmt`/`cargo clippy` the same way
          # `make check`'s `fmt`/`clippy` targets do for a local checkout.
          fmtCheck = pkgs.rustPlatform.buildRustPackage {
            pname = "d2b-provider-toolkit-fmt-check";
            version = "0.1.0";
            src = distributionSource;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.rustfmt ];
            buildPhase = ''
              runHook preBuild
              CARGO_NET_OFFLINE=true cargo fmt --all -- --check
              runHook postBuild
            '';
            installPhase = ''
              touch "$out"
            '';
            doCheck = false;
          };
          clippyCheck = pkgs.rustPlatform.buildRustPackage {
            pname = "d2b-provider-toolkit-clippy-check";
            version = "0.1.0";
            src = distributionSource;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
              runHook postBuild
            '';
            installPhase = ''
              touch "$out"
            '';
            doCheck = false;
          };
          # Hermetically exercises tests/check-distribution-policy.sh (which
          # is otherwise only run ad hoc via `make policy-test`, and was not
          # wired into any required CI/flake check). The regression test's
          # fixture builder walks `git ls-files`, mirroring how a real
          # checkout treats `canonical/d2b` as an (excluded) submodule
          # gitlink; distributionSource has no `.git` at all, so a throwaway
          # commit is made around a writable copy of it here purely so the
          # test's existing git-based fixture logic runs unmodified in both
          # a real checkout and this hermetic sandbox.
          policyCheck = pkgs.runCommand "d2b-provider-toolkit-policy-check" {
            nativeBuildInputs = [
              pkgs.bash
              pkgs.cargo
              pkgs.coreutils
              pkgs.findutils
              pkgs.git
              pkgs.gnugrep
              pkgs.gnused
              pkgs.gnutar
              pkgs.jq
              pkgs.rustc
            ];
          } ''
            cp -R ${distributionSource}/. candidate
            chmod -R u+w candidate
            git -C candidate init -q
            echo "/canonical/d2b/" >> candidate/.git/info/exclude
            git -C candidate add -A
            git -C candidate \
              -c user.email="checks.policy@d2b-provider-toolkit.invalid" \
              -c user.name="d2b-provider-toolkit checks.policy" \
              commit -q -m "candidate snapshot for checks.policy"
            HOME="$TMPDIR" CARGO_NET_OFFLINE=true \
              bash candidate/tests/check-distribution-policy.sh
            touch "$out"
          '';
        in
        {
          default = toolkit;
          d2b-provider-toolkit = toolkit;
          sourceArchive = sourceArchive;
          contractDocs = contractDocs;
          releasePackagingCheck = releasePackagingCheck;
          fmtCheck = fmtCheck;
          clippyCheck = clippyCheck;
          policyCheck = policyCheck;
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
          fmt = packages.fmtCheck;
          clippy = packages.clippyCheck;
          policy = packages.policyCheck;
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
