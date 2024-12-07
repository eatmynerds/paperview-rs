{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fp.url = "github:hercules-ci/flake-parts";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    { fp, ... }@inputs:
    fp.lib.mkFlake
      {
        inherit inputs;
      }
      {
        systems = [ "x86_64-linux" ];
        imports = [ inputs.devenv.flakeModule ];

        perSystem =
          { pkgs, lib, ... }:
          {
            devShells.default = pkgs.mkShell.override { stdenv = pkgs.llvmPackages.stdenv; } {
              buildInputs = with pkgs; [
                pkg-config
                libclang.lib
                xorg.libX11
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
                xorg.libXinerama
                xorg.libXxf86vm
                xorg.libXScrnSaver
                xorg.libXtst
                xorg.libXi
                libxkbcommon
                wayland
                wayland-protocols
                mesa
                imlib2.dev
                llvmPackages.libcxxStdenv
              ];

              shellHook = ''
                export BINDGEN_EXTRA_CLANG_ARGS="-I${pkgs.llvmPackages.libclang.lib}/lib/clang/${lib.getVersion pkgs.libclang}/include $NIX_CFLAGS_COMPILE"
                export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
              '';
            };
          };
      };
}
