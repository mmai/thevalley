# TODO 

Créer Game, Phase dans projet _game_ (pas protocol si non nécessaire), sur le modèle de leval_scala.

* tour de jeu
  - influence
    - interface choix 3 options
      - jouer arcane de la main
      - engendrer un être non déjà présent
      - éduquer un être
        - élévation
        - échange
  - actes
    - pour chaque être :
      - choix ressource cardinale 
      - si Coeur : maj majesté 
      - si Pouvoir (trèfle) : pioche et choix carte à regarder
      - si Arme (pique) ou Esprit (esprit)
        - choix adversaire 
        - résolution attaque 
        - maj êtres en fonction du résultat
  - source : 
    - retourne les cartes face contre terre
    - pioche

* fin de partie ?
