{
  description = "SyncClassroom Tauri dev environment";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        pythonEnv = pkgs.python3.withPackages (ps: [ ps.pillow ]);

      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            # ── Python（图标生成）────────────────────────────
            pythonEnv

            # ── just（任务运行器）────────────────────────────
            pkgs.just
          ];

          shellHook = ''

            if ! command -v rustup &>/dev/null; then
              echo "⚠️  rustup not found — install: https://rustup.rs"
            fi

            if ! command -v cargo-tauri &>/dev/null; then
              echo "ℹ️  tauri-cli not installed, run: just setup"
            fi

            echo ""
            echo "╔══════════════════════════════════════════╗"
            echo "║   SyncClassroom dev environment ready    ║"
            echo "╠══════════════════════════════════════════╣"
            printf "║  python  %-32s║\n" "$(python3 --version 2>&1)"
            printf "║  cargo   %-32s║\n" "$(cargo --version 2>&1 || echo 'not found')"
            printf "║  just    %-32s║\n" "$(just --version 2>&1)"
            echo "╠══════════════════════════════════════════╣"
            echo "║  first time?  just setup                 ║"
            echo "║  dev teacher: just dev-teacher            ║"
            echo "║  build all:   just build-all             ║"
            echo "╚══════════════════════════════════════════╝"
            echo ""
          '';
        };
      }
    );
}
