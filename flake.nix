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
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    ,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems
      (
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

          runtimeLibs = with pkgs; [
            wayland
            libxkbcommon
            libx11
            libxcb
            libxcursor
            libxi
            libxrandr
            vulkan-loader
            libGL
            fontconfig
            freetype
            gtk3
            glib
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
            version = "2.0.4";

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
            nativeBuildInputs =
              nativeBuildInputs
              ++ (with pkgs; [
                rustToolchain
                git
                cargo-deb
              ]);

            shellHook = ''
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"
              export PKG_CONFIG_PATH="${
                pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" runtimeLibs
              }:$PKG_CONFIG_PATH"
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
        { config
        , lib
        , pkgs
        , ...
        }:
        let
          cfg = config.programs.nyx;
        in
        {
          options.programs.nyx = {
            enable = lib.mkEnableOption "Nyx Mihomo/Clash GUI";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              description = "The Nyx package to use.";
            };
            tunMode = lib.mkEnableOption ''
              TUN mode. Wraps the Nyx binary with cap_net_admin/cap_net_raw/
              cap_net_bind_service so the mihomo core it spawns can create a TUN
              device without running as root'';
            profiles = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              example = [ "https://example.com/subscription" ];
              description = ''
                Subscription URLs imported automatically on launch, so profiles
                don't have to be added by hand. Idempotent: already-added URLs
                are skipped and a failed fetch is retried next launch. Profile
                names come from the subscription headers. Exported as the
                NYX_PROFILES environment variable.'';
            };
            profilesFile = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "/run/secrets/nyx-profiles";
              description = ''
                Path to a file with subscription URLs (whitespace/newline
                separated), imported like `profiles`. Use this for secret URLs
                rendered by sops/agenix so they never land in the Nix store.
                Exported as NYX_PROFILES_FILE.'';
            };
          };

          config =
            let
              needsWrap = cfg.profiles != [ ] || cfg.profilesFile != null;
              # Bake the declared profile env vars into the binary. sessionVariables
              # aren't reliably inherited by GUI-launched apps, so wrap instead —
              # this reaches Nyx no matter how the desktop starts it.
              wrapped = pkgs.symlinkJoin {
                name = "nyx-with-profiles";
                paths = [ cfg.package ];
                nativeBuildInputs = [ pkgs.makeWrapper ];
                postBuild = ''
                  wrapProgram $out/bin/nyx \
                    ${
                      lib.optionalString (
                        cfg.profiles != [ ]
                      ) "--set NYX_PROFILES ${lib.escapeShellArg (lib.concatStringsSep " " cfg.profiles)}"
                    } \
                    ${lib.optionalString (
                      cfg.profilesFile != null
                    ) "--set NYX_PROFILES_FILE ${lib.escapeShellArg cfg.profilesFile}"}
                '';
              };
              runPackage = if needsWrap then wrapped else cfg.package;
            in
            lib.mkIf cfg.enable {
              environment.systemPackages = [ runPackage ];
              programs.dconf.enable = lib.mkDefault true;
              services.gnome.gnome-keyring.enable = lib.mkDefault true;

              # Caps live on the security wrapper; it raises them into the ambient
              # set and execs runPackage, so the core Nyx spawns inherits them.
              security.wrappers = lib.mkIf cfg.tunMode {
                nyx = {
                  owner = "root";
                  group = "root";
                  capabilities = "cap_net_bind_service,cap_net_raw,cap_net_admin=+ep";
                  source = "${runPackage}/bin/nyx";
                };
              };
            };
        };
    };
}
