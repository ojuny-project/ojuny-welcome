{
  description = "First-boot welcome menu for Ojuny";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nix-settings-gui.url = "github:ojuny-project/nix-settings-gui";
    tinted-theming = {
      url = "github:tinted-theming/schemes";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      nix-settings-gui,
      tinted-theming,
    }:
    let
      binaryInfo =
        if builtins.pathExists ./binary-url.json then
          builtins.fromJSON (builtins.readFile ./binary-url.json)
        else
          null;

      desktopPostInstall = ''
        install -Dm644 ${./org.ojuny.Welcome.desktop} "$out/share/applications/org.ojuny.Welcome.desktop"
      '';

      mkBinary =
        {
          system,
          tag ? (
            if binaryInfo == null then
              throw "ojuny-welcome: binary-url.json is missing"
            else
              binaryInfo.latest
          ),
          package ? "ojuny-welcome",
        }:
        let
          release =
            if binaryInfo == null then
              throw "ojuny-welcome: binary-url.json is missing"
            else
              binaryInfo.releases.${tag}
                or (throw "ojuny-welcome: release '${tag}' is not recorded in binary-url.json");
          packageInfo =
            release.packages.${package}
              or (throw "ojuny-welcome: package '${package}' is not recorded in release '${tag}'");
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.callPackage ./nix/binary.nix {
          name = package;
          program = packageInfo.program or package;
          inherit (packageInfo) file;
          info = {
            inherit (binaryInfo) repo system;
            inherit (release) tag version;
            inherit (packageInfo) url hash;
          };
          postInstall = desktopPostInstall;
        };
    in
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        clean =
          src:
          lib.cleanSourceWith {
            inherit src;
            filter =
              path: _type:
              let
                base = baseNameOf path;
              in
              base != "target" && base != ".git" && base != "result";
          };

        src = pkgs.runCommand "ojuny-welcome-src" { } ''
          mkdir -p $out
          cp -r ${clean ./.}/. $out/
          cp -r ${clean nix-settings-gui.outPath} $out/nix-settings-gui
          sed -i 's|path = "../nix-settings-gui"|path = "nix-settings-gui"|' $out/Cargo.toml
        '';

        runtimePath = lib.makeBinPath (
          with pkgs;
          [
            wlr-randr
            nixfmt
          ]
        );

        package = pkgs.rustPlatform.buildRustPackage {
          pname = "ojuny-welcome";
          version = "0.1.0";
          src = src;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            mold
          ];
          buildInputs = with pkgs; [
            gtk4
            libadwaita
            glib
          ];

          RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
          TINTED_SCHEMES_DIR = "${tinted-theming}";
          doCheck = true;

          preFixup = ''
            gappsWrapperArgs+=(--prefix PATH : "${runtimePath}")
          '';

          postInstall = ''
            install -Dm644 ${./org.ojuny.Welcome.desktop} \
              "$out/share/applications/org.ojuny.Welcome.desktop"
          '';

          meta = with lib; {
            description = "First-boot welcome menu for Ojuny";
            mainProgram = "ojuny-welcome";
            license = licenses.gpl3Plus;
            platforms = platforms.linux;
          };
        };
      in
      {
        packages = {
          inherit package;
          default = package;
        }
        // (
          if binaryInfo != null && binaryInfo.system == system && binaryInfo.latest != "" then
            {
              ojuny-welcome-binary = mkBinary { inherit system; };
            }
          else
            { }
        );

        apps.default = {
          type = "app";
          program = "${package}/bin/ojuny-welcome";
        };

        apps.ci = {
          type = "app";
          program = "${
            pkgs.writeShellApplication {
              name = "ci";
              runtimeInputs = [ pkgs.nix ];
              text = ''
                exec nix flake check "$@"
              '';
            }
          }/bin/ci";
        };

        checks.default = pkgs.runCommand "check-ojuny-welcome" { } ''
          echo ${package} >/dev/null
          touch $out
        '';

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
          ];
          buildInputs = with pkgs; [
            gtk4
            libadwaita
            glib
            wlr-randr
            nixfmt
          ];
          TINTED_SCHEMES_DIR = "${tinted-theming}";
        };
      }
    )
    // {
      hydraJobs = self.checks;
      lib.mkBinary = mkBinary;
      nixosModules.default = import ./modules/nixos.nix self;
      nixosModules.ojuny-welcome = import ./modules/nixos.nix self;
    };
}
