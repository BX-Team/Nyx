{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.nyx;
  caps = [
    "CAP_NET_ADMIN"
    "CAP_NET_RAW"
    "CAP_NET_BIND_SERVICE"
    "CAP_SYS_TIME"
    "CAP_SYS_PTRACE"
    "CAP_DAC_READ_SEARCH"
    "CAP_DAC_OVERRIDE"
    "CAP_CHOWN"
    "CAP_FOWNER"
  ];
in
{
  options.programs.nyx = {
    enable = lib.mkEnableOption "Nyx, a desktop GUI for the Mihomo proxy core";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.nyx;
      defaultText = lib.literalExpression "nyx.packages.\${system}.nyx";
      description = "The Nyx package to install.";
    };

    service = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Run the privileged mihomo supervisor as a declarative unit. Nyx then
          never asks polkit for anything, and TUN mode works out of the box.
        '';
      };

      user = lib.mkOption {
        type = lib.types.str;
        example = "alice";
        description = ''
          The only user allowed to drive the supervisor over its socket. This is
          the account you run the Nyx GUI from.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    systemd.services.nyx = lib.mkIf cfg.service.enable {
      description = "Nyx Service (mihomo core supervisor)";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network.target"
        "NetworkManager.service"
        "systemd-networkd.service"
        "iwd.service"
      ];
      serviceConfig = {
        Type = "simple";
        ExecStart = "${lib.getExe cfg.package} --nyx-service --nyx-service-owner ${cfg.service.user}";
        Restart = "always";
        RestartSec = 2;
        RuntimeDirectory = "nyx";
        RuntimeDirectoryMode = "0755";
        LimitNPROC = 500;
        LimitNOFILE = 1000000;
        CapabilityBoundingSet = caps;
        AmbientCapabilities = caps;
      };
    };

    environment.etc."nyx/service-managed" = lib.mkIf cfg.service.enable {
      text = "nixos\n";
    };
  };
}
