pkgs:
pkgs.writeShellApplication {
  name = "nix3-fmt-wrapper";

  runtimeInputs = [
    pkgs.alejandra
    pkgs.fd
    pkgs.prettier
  ];

  text = ''
    # Format Nix files with Alejandra
    fd "$@" -t f -e nix -x alejandra -q '{}'

    # Format CSS & Javascript files with Prettier
    fd "$@" -t f -e css -e js -x prettier -w '{}'
  '';
}
