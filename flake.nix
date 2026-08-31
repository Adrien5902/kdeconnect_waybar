{
  description = "KDE Connect Waybar module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          inherit (manifest) version;
          src = pkgs.lib.cleanSource ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          doCheck = false;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.makeWrapper
          ];
          buildInputs = [ pkgs.dbus ];

          postInstall = ''
            wrapProgram $out/bin/kdeconnect_waybar \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath [ pkgs.dbus ]}
            mkdir -p $out/share/kdeconnect_waybar
            mkdir -p $TMPDIR/kdeconnect_waybar
            HOME=$TMPDIR XDG_CONFIG_HOME=$TMPDIR $out/bin/kdeconnect_waybar gen_schema
            cp $TMPDIR/kdeconnect_waybar/config.schema.json $out/share/kdeconnect_waybar/config.schema.json
          '';
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.rustc
            pkgs.cargo
          ];
          buildInputs = [ pkgs.dbus ];
        };
      }
    )
    // {
      nixosModules.default = { pkgs, ... }: {
        environment.systemPackages = [ self.packages.${pkgs.system}.default ];
      };

      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.kdeconnect-waybar;
          pkg = self.packages.${pkgs.system}.default;
          schema = builtins.fromJSON (builtins.readFile "${pkg}/share/kdeconnect_waybar/config.schema.json");
          configDef = schema.definitions.Config or schema."$defs".Config;

          jsonSchemaToNixType =
            prop:
            if prop ? "$ref" || prop ? anyOf || prop ? oneOf then
              lib.types.nullOr lib.types.str
            else
              switchType (prop.type or null) prop;

          switchType =
            type: prop:
            if type == "string" then
              lib.types.str
            else if type == "integer" then
              lib.types.int
            else if type == "number" then
              lib.types.number
            else if type == "boolean" then
              lib.types.bool
            else if type == "array" then
              lib.types.listOf (jsonSchemaToNixType (prop.items or { type = "string"; }))
            else if type == "object" then
              lib.types.attrsOf (
                if prop ? additionalProperties && builtins.isAttrs prop.additionalProperties then
                  jsonSchemaToNixType prop.additionalProperties
                else
                  lib.types.str
              )
            else
              lib.types.anything;

          toOption =
            name: prop:
            lib.mkOption {
              type = lib.types.nullOr (jsonSchemaToNixType prop);
              default = prop.default or null;
              description = prop.description or "Option ${name}";
            };

          generatedOptions = lib.mapAttrs toOption configDef.properties;
        in
        {
          options.programs.kdeconnect-waybar = {
            enable = lib.mkEnableOption "kdeconnect_waybar";
            settings = lib.mkOption {
              type = lib.types.submodule { options = generatedOptions; };
              default = { };
              description = "kdeconnect_waybar settings";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ pkg ];
            xdg.configFile."kdeconnect_waybar/config.json".text = builtins.toJSON {
              configs = [ (lib.filterAttrs (_: v: v != null) cfg.settings) ];
            };
          };
        };
    };
}
