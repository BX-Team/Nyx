{
  description = "Nyx — Mihomo/Clash GUI built with Tauri 2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook
          bun
          nodejs_22
          rustToolchain
          cargo-tauri
          patchelf
        ];

        buildInputs = with pkgs; [
          # Tauri runtime deps on Linux
          webkitgtk_4_1
          libsoup_3
          gtk3
          glib
          glib-networking
          cairo
          pango
          gdk-pixbuf
          atk
          harfbuzz
          librsvg
          libayatana-appindicator
          openssl
          dbus
          xdotool
        ];

        # Frontend node_modules as a fixed-output derivation so cargo-tauri
        # can pick up `dist/` from `bun run web:build`.
        bunDeps = pkgs.stdenvNoCC.mkDerivation {
          pname = "nyx-bun-deps";
          version = "1.0.1";

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              let
                base = baseNameOf (toString path);
              in
              base == "package.json" || base == "bun.lock" || base == "patches";
          };

          nativeBuildInputs = [ pkgs.bun ];

          dontConfigure = true;
          dontFixup = true;

          buildPhase = ''
            export HOME=$(mktemp -d)
            bun install --frozen-lockfile --no-progress
          '';

          installPhase = ''
            mkdir -p $out
            cp -r node_modules $out/
          '';

          outputHashMode = "recursive";
          outputHashAlgo = "sha256";
          # Run `nix build .#packages.<system>.bunDeps` once with lib.fakeHash
          # to compute the real hash, then paste it here.
          outputHash = pkgs.lib.fakeHash;
        };

        nyx = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "nyx";
          version = "1.0.1";

          src = pkgs.lib.cleanSource ./.;

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit (finalAttrs) src;
            sourceRoot = "${finalAttrs.src.name or "source"}/src-tauri";
            # Run `nix build` once with lib.fakeHash to learn the real hash.
            hash = pkgs.lib.fakeHash;
          };

          inherit nativeBuildInputs buildInputs;

          nativeBuildInputs = nativeBuildInputs ++ [
            pkgs.rustPlatform.cargoSetupHook
          ];

          # Link prefetched node_modules in before any build step.
          postPatch = ''
            ln -s ${bunDeps}/node_modules ./node_modules
          '';

          # Tauri's `beforeBuildCommand` calls `bun run web:build`, so bun
          # must be on PATH and node_modules has to be linked.
          buildPhase = ''
            runHook preBuild

            export HOME=$(mktemp -d)
            export XDG_CACHE_HOME=$HOME/.cache

            # cargo-tauri runs `cargo build --release` itself
            cd src-tauri
            cargo tauri build --bundles deb --no-bundle
            # If you want a .deb in the result, drop --no-bundle and add deb-output below.

            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall

            mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor
            install -Dm755 target/release/nyx $out/bin/nyx

            # Desktop entry
            cat > $out/share/applications/nyx.desktop <<EOF
            [Desktop Entry]
            Name=Nyx
            Comment=Mihomo/Clash GUI
            Exec=$out/bin/nyx
            Icon=nyx
            Terminal=false
            Type=Application
            Categories=Network;Utility;
            StartupWMClass=Nyx
            EOF

            # Icons
            for size in 32 128; do
              if [ -f icons/''${size}x''${size}.png ]; then
                install -Dm644 icons/''${size}x''${size}.png \
                  $out/share/icons/hicolor/''${size}x''${size}/apps/nyx.png
              fi
            done

            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Mihomo/Clash GUI built with Tauri 2";
            homepage = "https://github.com/BX-Team/Nyx";
            license = licenses.gpl3Plus;
            platforms = supportedSystems;
            mainProgram = "nyx";
          };
        });
      in
      {
        packages = {
          default = nyx;
          inherit nyx bunDeps;
        };

        apps.default = {
          type = "app";
          program = "${nyx}/bin/nyx";
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            git
            bashInteractive
          ]);

          # WebKitGTK on NixOS needs these env vars at runtime.
          shellHook = ''
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
            export WEBKIT_DISABLE_COMPOSITING_MODE=1
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" buildInputs}:$PKG_CONFIG_PATH"

            echo "Nyx dev shell ready."
            echo "  bun install            # fetch JS deps"
            echo "  bun run tauri:dev      # run the app"
            echo "  bun run tauri:build    # build a release bundle"
          '';
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    )
    // {
      # NixOS module: `imports = [ inputs.nyx.nixosModules.default ];`
      nixosModules.default =
        { config, lib, pkgs, ... }:
        let
          cfg = config.programs.nyx;
        in
        {
          options.programs.nyx = {
            enable = lib.mkEnableOption "Nyx Mihomo/Clash GUI";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              description = "The Nyx package to use.";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];

            # Nyx binds the mihomo proxy on a high port; tweak as needed.
            networking.firewall.allowedTCPPorts = lib.mkDefault [ ];

            # WebKitGTK + tray icon needs these.
            programs.dconf.enable = lib.mkDefault true;
            services.gnome.gnome-keyring.enable = lib.mkDefault true;
          };
        };
    };
}
