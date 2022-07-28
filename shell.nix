{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell { buildInputs = with pkgs; [ xorg.libxcb pkgconfig xorg.libX11 ]; }
