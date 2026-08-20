{
  description = "Nyx — Mihomo/Clash GUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;
      eachSystem = f: lib.foldl' lib.recursiveUpdate { } (map f (import systems));
    in
    eachSystem (
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
          version = "2.1.0";

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
            description = "Mihomo/Clash GUI";
            homepage = "https://github.com/BX-Team/Nyx";
            license = licenses.gpl3Plus;
            platforms = import systems;
            mainProgram = "nyx";
          };
        };
      in
      {
        packages.${system} = {
          default = nyx;
          inherit nyx;
        };

        apps.${system}.default = {
          type = "app";
          program = "${nyx}/bin/nyx";
        };

        devShells.${system}.default = pkgs.mkShell {
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

        formatter.${system} = pkgs.nixfmt-rfc-style;
      }
    )
    // {
      nixosModules.nyx = import ./nix/module.nix { inherit self; };
      nixosModules.default = self.nixosModules.nyx;
    };
}
