# Make the behaviour of `nix-shell` consistent with the one of `nix develop`
{system ? builtins.currentSystem}: let
  ndg = import ./.;
in
  ndg.devShells.${system}.default
