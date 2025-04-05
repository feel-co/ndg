(import (
  let
    lock = builtins.fromJSON (builtins.readFile ./flake.lock);
    flakeCompatNode = lock.nodes.${lock.nodes.root.inputs.flake-compat}.locked;
  in
    fetchTarball {
      url = "https://github.com/${flakeCompatNode.owner}/${flakeCompatNode.repo}/archive/${flakeCompatNode.rev}.tar.gz";
      sha256 = flakeCompatNode.narHash;
    }
) {src = ./.;})
.shellNix
