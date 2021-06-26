use std::collections::BTreeMap;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

use thevalley_game::{NB_PLAYERS, cards, pos, trick, being, star, strength};
use webgame_protocol::{GameState, PlayerInfo, ProtocolErrorKind};
use crate::{ ProtocolError, DebugOperation, ValleyVariant };

use crate::player::{PlayerRole, GamePlayerState};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Phase {
    Influence,
    Act,
    Source,
}

/// A dealed card for each player (star)
/// used in twilight to determine the first player
/// there can be many if cards dealed are of same strength,
/// thus the Vec in the Status struct to keep the history of these deals
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StarsCards (
    #[serde_as(as = "BTreeMap<DisplayFromStr, _>")]
    BTreeMap<pos::PlayerPos, cards::Card>
);

impl From<BTreeMap<pos::PlayerPos, cards::Card>> for StarsCards {
    fn from(bt: BTreeMap<pos::PlayerPos, cards::Card>) -> Self {
        StarsCards(bt)
    }
}

impl Deref for StarsCards {
    type Target = BTreeMap<pos::PlayerPos, cards::Card>;
    fn deref(&self) -> &BTreeMap<pos::PlayerPos, cards::Card> {
        &self.0 // We just extract the inner element
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Pregame,
    Twilight(pos::PlayerPos, Vec<StarsCards>),
    Playing(pos::PlayerPos, Phase),
    Endgame,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ValleyGame {
    nb_players: u8,
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
            nb_players: 2,
            players: BTreeMap::new(),
            status: Status::Pregame,
            stars: BTreeMap::new(),
            source: cards::Deck::default(),
            river: cards::Deck::default(),
            revealed: vec![],
        }
    }
}

impl GameState for ValleyGame {
    type PlayerPos = pos::PlayerPos;
    type PlayerRole = PlayerRole;

    type GamePlayerState = GamePlayerState;
    type Snapshot = GameStateSnapshot;

    type Operation = DebugOperation;
    type VariantParameters = VariantSettings;

    fn set_variant(&mut self, variant: ValleyVariant) {
        self.nb_players = variant.parameters.nb_players;
        // self.deal = Deal::new(pos::PlayerPos::from_n(0, self.nb_players));
        // self.first = pos::PlayerPos::from_n(0, self.nb_players);
    }
    
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
        let mut newpos = pos::PlayerPos::from_n(nb_players, self.nb_players);

        for p in 0..self.nb_players {
            let position = pos::PlayerPos::from_n(p as usize, self.nb_players);
            if !self.position_taken(position){
                newpos = position.clone();
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

    fn get_player_role(&self, player_id: Uuid) -> Option<PlayerRole>{
        self.players.get(&player_id).map(|p| p.role)
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
        let pos = self.players[&player_id].pos;
        let mut hand: cards::Hand = cards::Hand::default();
        let stars = self.stars.iter().map(|(uuid, star)| {
            if player_id == *uuid {
                hand = star.get_hand();
            }
            star.make_snapshot(player_id == *uuid, &self.revealed)
        }
        ).collect();
        GameStateSnapshot {                                          
            nb_players: self.nb_players,
            players,                                                 
            stars,                                                   
            hand,
            pos,
            status: self.status.clone(),
            river: self.river.clone(),
            source_count: self.source.len() as u8,
        }                                                            
    }                                                                

    fn set_player_ready(&mut self, player_id: Uuid) -> bool {
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.ready = true;
            if self.status == Status::Pregame {
                player_state.role = PlayerRole::PreDeal;

                // Check if we start the game
                let mut count = 0;
                for player in self.players.values() {
                    if player.role == PlayerRole::PreDeal {
                        count = count + 1;
                    }
                }
                if count == self.nb_players {
                    self.init_game();
                    return true;
                }           
            }               
        }                   
        false
    }

    fn update_init_state(&mut self) -> bool {
        self.next();
        false
    }

    fn set_player_not_ready(&mut self, player_id: Uuid) {
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.ready = false;
        }
    }

    fn manage_operation(&mut self, operation: Self::Operation) {
        match operation {
            Self::Operation::SetSeed(seed) => {
                //TODO
                // let (hands, river) = tarotgame::deal_seeded_hands(seed, self.nb_players as usize);
            },
            Self::Operation::ShowState => {
                println!("Debug state : {}", serde_json::to_string(self).unwrap());
            }
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

        stars.into_iter().for_each(|(uuid, star)| {
                println!("star {}", star.get_hand().to_string());

                self.stars.insert(uuid, star);
            });

        self.source = source;
        let (first, last_cards) = self.do_twilight();
        self.status = match first {
            Some(pos_first) => Status::Twilight(pos_first, last_cards),
            None => Status::Endgame
        };
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

    pub fn do_twilight(&mut self) -> (Option<pos::PlayerPos>, Vec<StarsCards>){
    // pub fn do_twilight(&mut self) -> (Option<pos::PlayerPos>, Vec<BTreeMap<pos::PlayerPos, cards::Card>>){
        let mut drawn_cards = vec![];
        let mut first: Option<pos::PlayerPos> = None; // First player to play
        while !first.is_some() && !self.source.is_empty() {
            println!("twilight, stars count: {}, source count: {}", self.stars.len(), self.source.len());
            println!("source: {}", self.source.to_string());

            let mut source_cards: Vec<cards::Card> = vec![];
            for _ in 0..self.stars.len() {
                source_cards.push(self.source.draw());
            }

            let last_cards: StarsCards = self.stars.iter_mut()
            // let last_cards: BTreeMap<pos::PlayerPos, cards::Card> = self.stars.iter_mut()
                .zip(source_cards.into_iter())
                .map(|((_, star), last_card)| {
                    star.add_to_hand(last_card);
                    // println!("last card: {}", last_card.to_string());
                    (star.get_pos(), last_card)
                // }).collect();
                // }).collect().into();
                }).collect::<BTreeMap<pos::PlayerPos, cards::Card>>().into();

           let max_card = last_cards.iter()
                .max_by(|(_, a), (_, b)| strength(**a).cmp(&strength(**b)));

            first = if let Some((_, card)) = max_card {
                // println!("maxcard: {} ({})", card.to_string(), strength(*card));
                // for (_, last) in last_cards.iter() {
                //     println!("lastcard: {} ({})", last.to_string(), strength(*last));
                // }
                //Check there is no ex-aequo
                let high_points: Vec<(&pos::PlayerPos, &cards::Card)> = last_cards.iter().filter(|(_, a)| strength(**a).eq(&strength(*card))).collect();
                if high_points.len() > 1 {
                    None
                } else {
                    max_card.map(|c| *c.0)
                }
            } else {
                None
            };

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
pub struct VariantSettings {
    pub nb_players: u8,
}
                                                                     
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]           
pub enum PlayEvent {                                                 
    Play( Uuid, cards::Card)                                         
}                                                                    

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameStateSnapshot {
    pub nb_players: u8,
    pub players: Vec<GamePlayerState>,     
    pub hand: cards::Hand,
    pub pos: pos::PlayerPos,
    pub status: Status,                               
    pub stars: Vec<star::StarSnapshot>,
    pub source_count: u8,
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

    pub fn star(&self) -> &star::StarSnapshot {
        self.stars.iter().find(|s| s.get_pos() == self.pos).unwrap()
    }

}

impl Default for GameStateSnapshot {
    fn default() -> GameStateSnapshot {
        GameStateSnapshot {
            nb_players: 2,
            players: vec![],
            hand: cards::Hand::default(),
            pos: pos::PlayerPos { pos: pos::AbsolutePos::P0, count: 2},
            status: Status::Pregame,
            stars: vec![],
            source_count: 34,
            river: cards::Deck::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::{cards, pos};

    #[test]
    fn test_game_snapshot() {
        let mut game = ValleyGame::default();
        let p1_id = Uuid::new_v4();
        game.add_player(PlayerInfo { id: p1_id, nickname: "toto".into() });
        game.add_player(PlayerInfo { id: Uuid::new_v4(), nickname: "titi".into() });
        game.init_game();

        let snapshot = game.make_snapshot(p1_id);
        // print!("snapshot: {:?}", snapshot);
        let s = serde_json::to_string(&snapshot);
        print!("{:?}", s);
        assert!( s.is_ok());

        // print!("{}", s.unwrap());
    }
}
