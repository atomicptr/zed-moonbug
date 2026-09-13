{
  pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  },
}:

let
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    targets = [ "wasm32-wasip2" ];
  };
in
pkgs.mkShell {
  packages = [
    rustToolchain
    pkgs.zed-editor
  ];

  RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust";
}
