{ pkgs ? import <nixpkgs> {} }:
with pkgs;
mkShell {
    nativeBuildInputs = [ pkg-config cargo rustc ];
    buildInputs = [
        udev libinput
        gobject-introspection
        pango atk gdk-pixbuf gtk3
        libayatana-appindicator
    ];
}
