{
  description = "Commander Blood media extraction and reverse-engineering tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          # Graphics runtime libraries that windowing crates (winit, minifb,
          # softbuffer) dlopen at startup. On Nix these live in isolated store
          # paths rather than a global /usr/lib, so they must be put on
          # LD_LIBRARY_PATH explicitly or the dlopen fails at EventLoop init.
          # (The engine's x11rb backend is pure-Rust protocol-over-socket and
          # needs none of these, but they make the shell work for any graphics
          # tooling and let winit/minifb run under Xvfb.)
          graphicsLibs = with pkgs; [
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
            libxkbcommon
            wayland
            libGL
            vulkan-loader
            alsa-lib
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              curl
              dosbox-x
              ffmpeg
              imagemagick
              jq
              nasm
              p7zip
              pkg-config
              alsa-lib
              libxcb
              vulkan-tools
              (python3.withPackages (ps: with ps; [ capstone pillow numpy ]))
              ripgrep
              rust-analyzer
              rustc
              rustfmt
              xdotool
              xorg-server
            ];

            RUST_BACKTRACE = "1";

            shellHook = ''
              export FFMPEG="${pkgs.ffmpeg}/bin/ffmpeg"
              export FFPROBE="${pkgs.ffmpeg}/bin/ffprobe"
              export SEVENZIP="${pkgs.p7zip}/bin/7z"
              export LD_LIBRARY_PATH="/run/opengl-driver/lib:${pkgs.lib.makeLibraryPath graphicsLibs}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
            '';
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt
      );
    };
}
