let
  lock = builtins.fromJSON (builtins.readFile ../flake.lock);
  nixpkgsNode = lock.nodes.${lock.nodes.root.inputs.nixpkgs}.locked;
  nixpkgs = fetchTarball {
    inherit (nixpkgsNode) url;
    sha256 = nixpkgsNode.narHash;
  };
in
  import nixpkgs {}
