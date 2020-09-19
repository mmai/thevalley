use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use thevalley_game::{NB_PLAYERS, cards, pos, deal, trick, being, star, strength};
use webgame_protocol::{GameState, PlayerInfo, ProtocolErrorKind};
use crate::{ ProtocolError };

use crate::deal::{Deal, DealSnapshot};
use crate::player::{PlayerRole, GamePlayerState};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Phase {
    Influence,
    Act,
    Source,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Pregame,
    Twilight(pos::PlayerPos, Vec<BTreeMap<pos::PlayerPos, cards::Card>>),
    Playing(pos::PlayerPos, Phase),
    Endgame,
}

pub struct ValleyGame {
    players: BTreeMap<Uuid, GamePlayerState>,     
    status: Status,                               
    stars: BTreeMap<Uuid, star::Star>,            
    source : cards::Deck,                         
    river: cards::Deck,                      
// rules : CoreRules,                             
//  beingsState : Map[Card, Being.State] = Map(), //reset each round
//  lookedCards : Set[(Card, Suit)] = Set(),
    revealed : Vec<cards::Card>
}

impl Default for ValleyGame {
    fn default() -> ValleyGame {
        ValleyGame {
            players: BTreeMap::new(),
            status: Status::Pregame,
            stars: BTreeMap::new(),
            source: cards::Deck::default(),
            river: cards::Deck::default(),
            revealed: vec![],
        }
    }
}

impl GameState< GamePlayerState, GameStateSnapshot> for ValleyGame {
    type PlayerPos = pos::PlayerPos;
    type PlayerRole = PlayerRole;

    fn is_joinable(&self) -> bool {
        self.status == Status::Pregame
    }
    
    fn get_players(&self) -> &BTreeMap<Uuid, GamePlayerState> {
        &self.players
    }

    fn add_player(&mut self, player_info: PlayerInfo) -> pos::PlayerPos {
        if self.players.contains_key(&player_info.id) {
            return self.players.get(&player_info.id).unwrap().pos;
        }

        //Default pos
        let nb_players = self.players.len();
        let mut newpos = pos::PlayerPos::from_n(nb_players);

        //TODO rendre générique
        for p in &[ pos::PlayerPos::P0,
        pos::PlayerPos::P1,
        ] {
            if !self.position_taken(*p){
                newpos = p.clone();
                break;
            }
        }

        let state = GamePlayerState {
            player: player_info,
            // pos: pos::PlayerPos::from_n(nb_players),
            pos: newpos,
            role: PlayerRole::Spectator,
            ready: false,
        };
        self.players.insert(state.player.id, state.clone());
        newpos
    }

    fn remove_player(&mut self, player_id: Uuid) -> bool {
        self.players.remove(&player_id).is_some()
    }

    fn set_player_role(&mut self, player_id: Uuid, role: PlayerRole) {
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.role = role;
        }
    }

    fn player_by_pos(&self, position: pos::PlayerPos) -> Option<&GamePlayerState> {
        self.players.iter().find(|(_uuid, player)| player.pos == position).map(|p| p.1)
    }

    // Creates a view of the game for a player
    fn make_snapshot(&self, player_id: Uuid) -> GameStateSnapshot {
        let mut players = vec![];
        for (&_other_player_id, player_state) in self.players.iter() {
            players.push(player_state.clone());
        }
        players.sort_by(|a, b| a.pos.to_n().cmp(&b.pos.to_n()));
        // let pos = self.players[&player_id].pos;
        let stars = self.stars.iter().map(|(uuid, star)|
            star.make_snapshot(player_id == *uuid, &self.revealed)
        ).collect();
        GameStateSnapshot {                                          
            players,                                                 
            status: self.status.clone(),
            river: self.river.clone(),
            stars,                                                   
        }                                                            
    }                                                                

    fn set_player_ready(&mut self, player_id: Uuid){
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.ready = true;
            if self.status != Status::Pregame {
                self.next();
            } else {
                player_state.role = PlayerRole::PreDeal;

                // Check if we start the game
                let mut count = 0;
                for player in self.players.values() {
                    if player.role == PlayerRole::PreDeal {
                        count = count + 1;
                    }
                }
                if count == NB_PLAYERS {
                    self.init_game();
                }           
                            
            }               
        }                   
    }

    fn set_player_not_ready(&mut self, player_id: Uuid) {
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.ready = false;
        }
    }

}

impl ValleyGame {
    fn init_game(&mut self){
        let (hands, source) = thevalley_game::deal_hands();
        let stars: Vec<(Uuid, star::Star)>  = self.players.iter().zip(hands.into_iter())
            .map(|((uuid, player), hand)| {
                (*uuid, star::Star::new(player.pos, *hand))
            }).collect();

        stars.into_iter().map(|(uuid, star)| {
                self.stars.insert(uuid, star);
            });

        self.source = source;
        let (first, last_cards) = self.do_twilight();
        // The 'unwrap_or' has no consequences : if 'first' is None (what are the odds ?), there are no cards left in the source, the game is thus already finished...
        self.status = Status::Twilight(first.unwrap_or(pos::PlayerPos::P0), last_cards);
    }

    fn position_taken(&self, position: pos::PlayerPos) -> bool {
        self.player_by_pos(position) != None
    }

    pub fn players_ready(&self) -> bool {
        !(self.players.iter().find(|(_, player)| player.ready == false) != None)
    }

    pub fn next(&mut self){
        match self.status {
            // let first = self.stars.iter()
            //     .map|(_, s)| s.) 
            Status::Pregame => { },
            Status::Twilight(first, _) => {
                self.status = Status::Playing(first, Phase::Influence);
            },
            Status::Playing(pos, Phase::Influence) => {
                self.status = Status::Playing(pos, Phase::Act);
            },
            Status::Playing(pos, Phase::Act) => {
                self.status = if self.source.is_empty() {
                    Status::Endgame
                } else {
                    Status::Playing(pos, Phase::Source)
                };
            }
            Status::Playing(pos, Phase::Source) => {
                self.status = Status::Playing(pos.next(), Phase::Influence);
            }
            Status::Endgame => { },
        }
    }

    pub fn do_twilight(&mut self) -> (Option<pos::PlayerPos>, Vec<BTreeMap<pos::PlayerPos, cards::Card>>){
        let mut drawn_cards = vec![];
        let mut first: Option<pos::PlayerPos> = None; // First player to play
        while !first.is_some() && !self.source.is_empty() {

            let mut source_cards: Vec<cards::Card> = vec![];
            for _ in 0..self.stars.len() {
                source_cards.push(self.source.draw());
            }

            let last_cards: BTreeMap<pos::PlayerPos, cards::Card> = self.stars.iter_mut()
                .zip(source_cards.into_iter())
                .map(|((_, star), last_card)| {
                    star.add_to_hand(last_card);
                    (star.pos(), last_card)
                }).collect();

            first = last_cards.iter()
                .max_by(|(_, a), (_, b)| strength(**a).cmp(&strength(**b)))
                .map(|c| *c.0);
            drawn_cards.push(last_cards);
        }

        (first, drawn_cards)
    }

    fn end_trick(&mut self) {
        for player in self.players.values_mut() {
            if player.role != PlayerRole::Spectator {
                player.ready = false;
            }
        }
    }

    fn end_last_trick(&mut self) {
        for player in self.players.values_mut() {
            if player.role != PlayerRole::Spectator {
                player.ready = false;
                player.role = PlayerRole::Unknown;
            }
        }
    }

}                                                                    
                                                                     
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]           
pub enum PlayEvent {                                                 
    Play( Uuid, cards::Card)                                         
}                                                                    

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameStateSnapshot {
    pub players: Vec<GamePlayerState>,     
    pub status: Status,                               
    pub stars: Vec<star::StarSnapshot>,
    pub river: cards::Deck,                      
}


impl webgame_protocol::GameStateSnapshot for GameStateSnapshot {

}

impl GameStateSnapshot {
    pub fn get_playing_pos(&self) -> Option<pos::PlayerPos> {
        match self.status {
            Status::Playing(pos, _) => Some(pos),
            _ => None
        }
    }

    pub fn pos_player_name(&self, pos: pos::PlayerPos) -> String {
        self.players.iter()
            .find(|p| p.pos == pos)
            .map(|found| &found.player.nickname)
            .unwrap() // panic on invalid pos
            .into()
    }

    pub fn current_player_name(&self) -> String {
        let found_name = self.get_playing_pos().map(|pos| {
            self.pos_player_name(pos)
        });

        if let Some(name) = found_name {
            format!("{}", name)
        } else {
            "".into()
        }
    }

}

impl Default for GameStateSnapshot {
    fn default() -> GameStateSnapshot {
        GameStateSnapshot {
            players: vec![],
            status: Status::Pregame,
            stars: vec![],
            river: cards::Deck::default()
        }
    }
}
