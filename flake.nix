{
  description = "Nyx — Mihomo/Clash GUI";

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
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Libraries gpui (X11/Wayland/Vulkan/font-kit) and the tray
        # (GTK/appindicator) need to link against and dlopen at runtime.
        runtimeLibs = with pkgs; [
          wayland
          libxkbcommon
          xorg.libX11
          xorg.libxcb
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          vulkan-loader
          libGL
          fontconfig
          freetype
          gtk3
          glib
          libayatana-appindicator
          xdotool
          openssl
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustPlatform.bindgenHook # gpui builds bindgen-based crates
          autoPatchelfHook
          makeWrapper
          wrapGAppsHook3
        ];

        nyx = rustPlatform.buildRustPackage {
          pname = "nyx";
          version = "2.0.1";

          src = pkgs.lib.cleanSource ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          inherit nativeBuildInputs;
          buildInputs = runtimeLibs;

          # gpui dlopens Vulkan/Wayland/GL at runtime; bake them into the rpath.
          runtimeDependencies = runtimeLibs;

          # Heavy GPU/UI crate graph: skip the (nonexistent) test suite.
          doCheck = false;

          postInstall = ''
            install -Dm644 installer/linux/nyx.desktop \
              $out/share/applications/nyx.desktop
            install -Dm644 assets/brand/logo.png \
              $out/share/icons/hicolor/512x512/apps/nyx.png
          '';

          meta = with pkgs.lib; {
            description = "Mihomo/Clash GUI (pure-Rust gpui app)";
            homepage = "https://github.com/BX-Team/Nyx";
            license = licenses.gpl3Plus;
            platforms = supportedSystems;
            mainProgram = "nyx";
          };
        };
      in
      {
        packages = {
          default = nyx;
          inherit nyx;
        };

        apps.default = {
          type = "app";
          program = "${nyx}/bin/nyx";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = runtimeLibs;
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            rustToolchain
            git
            cargo-deb
            cargo-generate-rpm
          ]);

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" runtimeLibs}:$PKG_CONFIG_PATH"
            echo "Nyx dev shell ready."
            echo "  cargo run             # run the app"
            echo "  cargo build --release # optimized binary"
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
            tunMode = lib.mkEnableOption ''
              TUN mode. Wraps the Nyx binary with cap_net_admin/cap_net_raw/
              cap_net_bind_service so the mihomo core it spawns can create a TUN
              device without running as root'';
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
            programs.dconf.enable = lib.mkDefault true;
            services.gnome.gnome-keyring.enable = lib.mkDefault true;

            # Give the Nyx binary the net capabilities; Nyx raises them into the
            # ambient set before spawning the core, which then inherits them.
            security.wrappers = lib.mkIf cfg.tunMode {
              nyx = {
                owner = "root";
                group = "root";
                capabilities = "cap_net_bind_service,cap_net_raw,cap_net_admin=+ep";
                source = lib.getExe cfg.package;
              };
            };
          };
        };
    };
}
