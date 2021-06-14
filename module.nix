{config, lib, pkgs, ...}:

with lib;

let
  cfg = config.services.thevalley;
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

        dbUri = mkOption {
          type = types.str;
          default = "/var/thevalley/thevalley_db";
          description = ''
            thevalley database URI.
          '';
        };

        archivesDirectory = mkOption {
          type = types.path;
          default = "/var/thevalley/archives";
          description = ''
            thevalley directory path where game archives are stored
          '';
        };

        archivageCheck = mkOption {
          type = types.int;
          default = 120;
          description = ''
            thevalley archivage check period in minutes.
          '';
        };

        archivageDelay = mkOption {
          type = types.int;
          default = 1440;
          description = ''
            thevalley retention period in minutes after wich games are archived
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

          proxy_headers_hash_max_size 512;
          proxy_headers_hash_bucket_size 128; 

          # config for websockets proxying (cf. http://nginx.org/en/docs/http/websocket.html)
          proxy_http_version 1.1;
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection $connection_upgrade;
        '';
        withSSL = cfg.protocol == "https";
        in {
          "${cfg.hostname}" = {
            enableACME = withSSL;
            forceSSL = false;
            locations = {
              "/" = { 
                extraConfig = proxyConfig;
                proxyPass = "http://thevalley-api/";
              };
            };
          };
        };
      };

      systemd.tmpfiles.rules = [
        "d ${cfg.archivesDirectory} 0755 ${cfg.user} ${cfg.group} - -"
        "d ${cfg.dbUri} 0755 ${cfg.user} ${cfg.group} - -"
      ];

      systemd.targets.thevalley = {
        description = "thevalley";
        wants = ["thevalley-server.service"];
      }; 
      systemd.services = 
      let serviceConfig = {
        User = "${cfg.user}";
        WorkingDirectory = "${pkgs.thevalley}";
      };
      in {
        thevalley-server = {
          description = "thevalley application server";
          partOf = [ "thevalley.target" ];

          serviceConfig = serviceConfig // { 
            ExecStart = ''${pkgs.thevalley}/bin/thevalley_server -d ${pkgs.thevalley-front}/ \
              -p ${toString cfg.apiPort} -u ${cfg.dbUri} \
              --archives-directory ${toString cfg.archivesDirectory} \
              --archive-check ${toString cfg.archivageCheck} \
              --archive-delay ${toString cfg.archivageDelay} \
              '';
          };

          wantedBy = [ "multi-user.target" ];
        };
      };

    };

    meta = {
      maintainers = with lib.maintainers; [ mmai ];
    };
  }
