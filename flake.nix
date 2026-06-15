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
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              curl
              dosbox-x
              ffmpeg
              jq
              nasm
              p7zip
              pkg-config
              (python3.withPackages (ps: with ps; [ capstone pillow numpy ]))
              ripgrep
              rust-analyzer
              rustc
              rustfmt
            ];

            RUST_BACKTRACE = "1";

            shellHook = ''
              export FFMPEG="${pkgs.ffmpeg}/bin/ffmpeg"
              export FFPROBE="${pkgs.ffmpeg}/bin/ffprobe"
              export SEVENZIP="${pkgs.p7zip}/bin/7z"
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
