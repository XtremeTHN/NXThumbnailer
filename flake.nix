{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    nativeBuildInputs = with pkgs; [
      rustc
      cargo
      meson
      ninja
      pkg-config
      rustPlatform.cargoSetupHook
    ];

    buildInputs = with pkgs; [
      glib
    ];

    pname = "nxthumbnail";
    version = "0.1.0";
    src = ./.;
  in {
    devShells.${system}.default = pkgs.mkShell {
      inherit nativeBuildInputs;
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      
      packages = with pkgs; [
        rust-analyzer
        clippy
        rustfmt
      ];
    };

    packages.${system}.default = pkgs.stdenv.mkDerivation {
      name = pname;
      inherit version src;

      cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
        inherit pname version src;
        hash = "sha256-Z+yvo4QN0anTmvWUrv4mVdEFyc8itu597TdbAxD9S2M=";
      };

      postInstall = ''
        substituteInPlace $out/share/thumbnailers/nxthumbnail.thumbnailer \
          --replace-fail '=nxthumbnail' "=$out/bin/nxthumbnail"
      '';

      inherit nativeBuildInputs buildInputs;
    };
  };
}
