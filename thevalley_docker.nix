{ pkgs ? import <nixpkgs> {} }:

with pkgs;
let
  port = "8080";
  thevalley = (import ./thevalley.nix) { pkgs = pkgs; };
  entrypoint = writeScript "entrypoint.sh" ''
    #!${stdenv.shell}
    IP=$(ip route get 1 | awk '{print $NF;exit}')
    echo "Starting server. Open your client on http://$IP:${port}"
    ${thevalley}/bin/thevalley_server -d ${thevalley}/front/ -a $IP -p ${port}
  '';
in
  dockerTools.buildImage {
    name = "mmai/thevalley";
    tag = "0.1.0";
    contents = [ busybox ];
    config = {
      Entrypoint = [ entrypoint ];
      ExposedPorts = {
        "${port}/tcp" = {};
      };
    };
  }
