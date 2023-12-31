{ pkgs ? import <nixpkgs> {} }:
with pkgs;
mkShell {
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [
        udev libinput libiio
        gobject-introspection
        pango atk gdk-pixbuf gtk3
        libayatana-appindicator
    ];
}
