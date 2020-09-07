{config, lib, pkgs, ...}:

with lib;

let
  cfg = config.services.thevalley;
  thevalley = (import ./thevalley.nix) { pkgs = pkgs; };
in 
  {

    options = {
      services.thevalley = {
        enable = mkEnableOption "thevalley";

        user = mkOption {
          type = types.str;
          default = "thevalley";
          description = "User under which thevalley is ran.";
        };

        group = mkOption {
          type = types.str;
          default = "thevalley";
          description = "Group under which thevalley is ran.";
        };

        protocol = mkOption {
          type = types.enum [ "http" "https" ];
          default = "https";
          description = ''
            Web server protocol.
          '';
        };


        hostname = mkOption {
          type = types.str;
          default = "tarot.localhost";
          description = "Public domain name of the thevalley web app.";
        };

        apiPort = mkOption {
          type = types.port;
          default = 8002;
          description = ''
            thevalley API Port.
          '';
        };

      };
    };

    config = mkIf cfg.enable {
      users.users.thevalley = mkIf (cfg.user == "thevalley") { group = cfg.group; };

      users.groups.thevalley = mkIf (cfg.group == "thevalley") {};

      services.nginx = {
        enable = true;
        appendHttpConfig = ''
          upstream thevalley-api {
          server localhost:${toString cfg.apiPort};
          }
        '';
        virtualHosts = 
        let proxyConfig = ''
          # global proxy conf
          proxy_set_header Host $host;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
          proxy_set_header X-Forwarded-Proto $scheme;
          proxy_set_header X-Forwarded-Host $host:$server_port;
          proxy_set_header X-Forwarded-Port $server_port;
          proxy_redirect off;

          # websocket support
          proxy_http_version 1.1;
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection $connection_upgrade;
        '';
        withSSL = cfg.protocol == "http";
        in {
          "${cfg.hostname}" = {
            enableACME = withSSL;
            forceSSL = withSSL;
            locations = {
              "/" = { 
                extraConfig = proxyConfig;
                proxyPass = "http://thevalley-api/";
              };
            };
          };
        };
      };

      systemd.targets.thevalley = {
        description = "thevalley";
        wants = ["thevalley-server.service"];
      }; 
      systemd.services = 
      let serviceConfig = {
        User = "${cfg.user}";
        WorkingDirectory = "${thevalley}";
      };
      in {
        thevalley-server = {
          description = "thevalley application server";
          partOf = [ "thevalley.target" ];

          serviceConfig = serviceConfig // { 
            ExecStart = ''${thevalley}/bin/thevalley_server -d ${thevalley}/front/ \
              -p ${toString cfg.apiPort}'';
          };

          wantedBy = [ "multi-user.target" ];
        };
      };

    };

    meta = {
      maintainers = with lib.maintainers; [ mmai ];
    };
  }
