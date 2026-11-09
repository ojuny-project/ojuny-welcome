self: {
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.ojuny-welcome;
in {
  options.services.ojuny-welcome = {
    enable = mkEnableOption "ojuny-welcome, the first-boot setup wizard";

    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "The ojuny-welcome package to use.";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];
    environment.etc."xdg/autostart/ojuny-welcome.desktop".source = ../ojuny-welcome.desktop;
  };
}
