{ pkgs, ... }:

/* 

  beaconDB is currently deployed on my home server. my home server's 
  configuration is not public and rather messy. when beaconDB is deployed to
  it's own dedicated server I hope to make the configuration for that
  public for transparency.

  relevant configs that are not public:

  - mariadb: using pkgs.mariadb_110 (currently 11.0.6, https://search.nixos.org/packages?channel=unstable&from=0&size=50&sort=relevance&type=packages&query=mariadb_110)
  - nginx: hsts, locations/routes

  please open an issue/reach out if you have any questions regarding current
  infrastructure configuration

*/

let
  config = ./production.toml;
in

{
  systemd = {
    timers.beacondb-process = {
      after = [ "network.target" ];
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnBootSec = "5m";
        OnUnitActiveSec = "5m";
        Unit = "beacondb-process.service";
      };
    };

    services = {
      beacondb = {
        after = [ "network.target" ];
        wantedBy = [ "multi-user.target" ];

        script = ''
          ${pkgs.beacondb}/bin/beacondb -c ${config} serve 8924
        '';

        serviceConfig = {
          Type = "simple";

          User = "beacondb";
          Restart = "on-failure";
        };
      };

      beacondb-process = {
        script = ''
          ${pkgs.beacondb}/bin/beacondb -c ${config} process
        '';

        serviceConfig = {
          Type = "oneshot";

          User = "beacondb";
          Restart = "on-failure";
        };
      };
    };
  };

  users.users.beacondb = {
    group = "beacondb";
    isSystemUser = true;
  };
  users.groups.beacondb = { };
}

