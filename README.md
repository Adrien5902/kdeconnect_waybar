<h1 align="center">KDE Connect Waybar</h1>
<p align="center">
    <b>
        A highly configurable <a href="https://kdeconnect.kde.org/">KDE Connect</a> module for <a href="https://github.com/Alexays/Waybar/">Waybar</a>
    </b>
</p>

<p align="center">
<img alt="Crates.io Version" src="https://img.shields.io/crates/v/kdeconnect_waybar?style=flat-square">
<img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/Adrien5902/kdeconnect_waybar/docs.yaml?style=flat-square&label=docs">
</p>

## 🔧 Installation

### Using `cargo` :

simply run the command :

```sh
cargo install kdeconnect_waybar
```

### Using `nix` :

#### Run directly

Execute the package without installing it permanently:

```sh
nix run github:Adrien5902/kdeconnect_waybar
```

<details>
<summary>NixOS Configuration</summary>

Add the flake to your inputs and import the module to add the package to `environment.systemPackages`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    kdeconnect_waybar.url = "github:Adrien5902/kdeconnect_waybar";
  };

  outputs = { self, nixpkgs, kdeconnect_waybar, ... }: {
    nixosConfigurations.yourHostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        kdeconnect_waybar.nixosModules.default
      ];
    };
  };
}
```

</details>

<details>
<summary>Home Manager Configuration</summary>

Add the flake to your inputs, import the Home Manager module, and configure your settings:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    kdeconnect_waybar.url = "github:Adrien5902/kdeconnect_waybar";
  };

  outputs = { self, nixpkgs, home-manager, kdeconnect_waybar, ... }: {
    homeConfigurations."username@hostname" = home-manager.lib.homeManagerConfiguration {
      pkgs = nixpkgs.legacyPackages."x86_64-linux";
      modules = [
        kdeconnect_waybar.homeManagerModules.default
        {
          programs.kdeconnect-waybar = {
            enable = true;
            settings = {
              configs = [
                {
                  name = "default-device";
                  format = "{DeviceName} {Battery}";
                  app_icons = {
                    YouTube = "󰗃";
                  };
                  update_interval_secs = 2.5;
                }
              ];
            };
          };
        }
      ];
    };
  };
}
```

</details>

## ⚙️ Configuration

See [docs 📄](https://adrien5902.github.io/kdeconnect_waybar/kdeconnect_waybar/) for customization and styling

## 🧭 Examples

> ![Preview](./assets/preview.png)

> My personal config :
>
> ![Preview](./assets/cyberpunk.png)

