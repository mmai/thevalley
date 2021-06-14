{
  description = "The Valley";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-21.05;

  outputs = { self, nixpkgs }:
  let
    systems = [ "x86_64-linux" "i686-linux" "aarch64-linux" ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system); 
    # Memoize nixpkgs for different platforms for efficiency.
    nixpkgsFor = forAllSystems (system:
      import nixpkgs {
        inherit system;
        overlays = [ self.overlay ];
      }
    );
  in {
    overlay = final: prev: {

      thevalley-front = final.stdenv.mkDerivation {
        name = "thevalley-front";
        src = ./client;
        installPhase = ''
          mkdir -p $out
          cp -R ./static/* $out
          cp ./dist/*.{css,js,wasm} $out
        '';
      };

      thevalley = with final; ( rustPlatform.buildRustPackage rec {
          name = "thevalley";
          version = "0.0.1";
          src = ./.;

          nativeBuildInputs = [ pkgconfig ];
          buildInputs = [ openssl ];

          cargoSha256 = "sha256-K1faj/H8mb4cdOFnvlm1ZUGkoRnVvKLWF7SUNLOGEYY=";

          meta = with pkgs.stdenv.lib; {
            description = "A online game of the valley";
            homepage = "https://github.com/mmai/thevalley";
            license = licenses.gpl3;
            platforms = platforms.unix;
            maintainers = with maintainers; [ mmai ];
          };
        });

      thevalley-docker = with final;
        let
          port = "8080";
          data_path = "/var/thevalley";
          db_uri = "${data_path}/thevalley_db";
          runAsRoot = ''
            mkdir -p ${data_path}/archives
          '';
          entrypoint = writeScript "entrypoint.sh" ''
            #!${stdenv.shell}
            IP=$(ip route get 1 | awk '{print $NF;exit}')
            echo "Starting server. Open your client on http://$IP:${port}"
            ${thevalley}/bin/thevalley_server -d ${thevalley-front}/ -a $IP -p ${port} -u ${db_uri} --archives-directory ${data_path}/archives
          '';
        in 
          dockerTools.buildImage {
            name = "mmai/thevalley";
            tag = "0.0.1";
            contents = [ busybox ];
            config = {
              Entrypoint = [ entrypoint ];
              ExposedPorts = {
                "${port}/tcp" = {};
              };
            };
          };

    };

    packages = forAllSystems (system: {
      inherit (nixpkgsFor.${system}) thevalley;
      inherit (nixpkgsFor.${system}) thevalley-front;
      inherit (nixpkgsFor.${system}) thevalley-docker;
    });

    defaultPackage = forAllSystems (system: self.packages.${system}.thevalley);


    # Use nixpkgs with oxalica rust-bin overlay (cf. https://github.com/NixOS/nixpkgs/issues/112535)
    # rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");


    devShell = forAllSystems (system: (import ./shell.nix { pkgs = nixpkgs.legacyPackages.${system}; }));

    # thevalley service module
    nixosModule = (import ./module.nix);

  };
}
