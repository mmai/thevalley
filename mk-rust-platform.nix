{ callPackage, fetchFromGitHub, makeRustPlatform }:

{ date, channel }:

let mozillaOverlay = fetchFromGitHub {
      owner = "mozilla";
      repo = "nixpkgs-mozilla";
      rev =  "e912ed483e980dfb4666ae0ed17845c4220e5e7c";
      sha256 =  "08fvzb8w80bkkabc1iyhzd15f4sm7ra10jn32kfch5klgl0gj3j3";
    };
    mozilla = callPackage "${mozillaOverlay.out}/package-set.nix" {};
    rustSpecific = (mozilla.rustChannelOf { inherit date channel; }).rust;

in makeRustPlatform {
  cargo = rustSpecific;
  rustc = rustSpecific;
}
