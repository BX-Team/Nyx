<div align="center">

<img src=".github/branding/logo.png" width="128" height="128" alt="Nyx Logo" />

# Nyx

A modern, lightweight desktop GUI for the [Mihomo](https://github.com/MetaCubeX/mihomo) proxy core. Manage profiles, proxy groups, rules and connections from a clean interface — with system proxy and TUN mode, connection inspector with per-process grouping and app icons, built-in profile editor, auto-updater and a polished UX.

[![Chat on Discord](https://cdn.jsdelivr.net/npm/@intergrav/devins-badges@3/assets/cozy/social/discord-plural_vector.svg)](https://discord.gg/qNyybSSPm5)
[![github](https://cdn.jsdelivr.net/npm/@intergrav/devins-badges@3/assets/cozy/available/github_vector.svg)](https://github.com/BX-Team/Nyx)

</div>

# Preview

![preview](.github/branding/preview.png)

# Installation

Grab the latest build from the [Releases page](https://github.com/BX-Team/Nyx/releases/latest).

## Windows (x86_64)

- **Installer:** `Nyx_<version>_x64-setup.exe` — run it and follow the prompts. On first launch Nyx asks for elevation to install the helper service required for TUN mode; accept it once and you are set.
- **Portable:** `Nyx-x86_64-windows.zip` — unzip anywhere and run `nyx.exe`. No install, settings live in your user data dir.

## Linux (x86_64)

Pick the package for your distro, or the portable tarball:

- **Debian/Ubuntu:** `Nyx_<version>_amd64.deb` — `sudo apt install ./Nyx_<version>_amd64.deb`
- **Fedora/RHEL:** `Nyx-<version>.x86_64.rpm` — `sudo dnf install ./Nyx-<version>.x86_64.rpm`
- **Arch:** `Nyx-<version>-x86_64.pkg.tar.xz` — `sudo pacman -U ./Nyx-<version>-x86_64.pkg.tar.xz`
- **Portable:** `Nyx-x86_64-linux.tar.gz` — extract and run `./nyx`

### Nix

Nyx ships a flake. Run it directly without installing:

```bash
nix run github:BX-Team/Nyx
```

Or add it to your own flake as an input:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nyx.url = "github:BX-Team/Nyx";
  };

  outputs = {
    self,
    nixpkgs,
    nyx,
    ...
  }: {
    ...
  };
}
```

Then add the package to your `environment.systemPackages` or `home.packages`:

```nix
# NixOS configuration
{
  pkgs,
  inputs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    inputs.nyx.packages.${pkgs.system}.nyx
  ];
}
```

```nix
# Home Manager configuration
{
  pkgs,
  inputs,
  ...
}: {
  home.packages = with pkgs; [
    inputs.nyx.packages.${pkgs.system}.nyx
  ];
}
```

To pull a **prebuilt** binary from the Cachix cache instead of compiling locally, add the substituter and its public key:

```nix
nix = {
  settings = {
    substituters = [
      "https://bx-team.cachix.org"
    ];
    trusted-public-keys = [
      "bx-team.cachix.org-1:tnGNc1rsS8QOav+VGxXCZzf/Y0/SGchOwVCCBA/eG6E="
    ];
  };
};
```

There is also a NixOS module: `imports = [ inputs.nyx.nixosModules.default ];` then `programs.nyx.enable = true;`.

## Build from source

Nyx is now a single pure-Rust [gpui](https://github.com/zed-industries/zed) application. The only hard requirement is a stable [Rust](https://www.rust-lang.org/tools/install) toolchain.

```bash
git clone https://github.com/BX-Team/Nyx.git
cd Nyx
cargo run             # run in development
cargo build --release # produce an optimized binary at target/release/nyx
```

On **Linux** you also need the gpui/tray system libraries. On Debian/Ubuntu:

```bash
sudo apt-get install -y \
  libgtk-3-dev libxdo-dev libayatana-appindicator3-dev \
  libxkbcommon-dev libwayland-dev \
  libx11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libfontconfig1-dev libfreetype6-dev \
  libvulkan-dev mesa-vulkan-drivers
```

Or just use the flake: `nix develop` drops you into a shell with everything wired up.


# License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

# Contributing

We welcome contributions to Nyx! If you have an idea for a new feature or found a bug, please feel free to submit a pull request. Before you start, please read our [contributing guidelines](CONTRIBUTING.md) to understand our contribution process.

# Credits

Nyx was based on or inspired by these projects:

- [MetaCubeX/mihomo](https://github.com/MetaCubeX/mihomo): A rule-based tunnel in Go.
- [DINGDANGMAOUP/mihomo-rs](https://github.com/DINGDANGMAOUP/mihomo-rs): A Rust SDK for Mihomo, manages versions, configs and other things.
- [zed-industries/zed](https://github.com/zed-industries/zed): Home of the [gpui](https://www.gpui.rs/) GPU-accelerated UI framework that Nyx is built on.
- [longbridge/gpui-component](https://github.com/longbridge/gpui-component): The gpui component library powering Nyx's widgets.
