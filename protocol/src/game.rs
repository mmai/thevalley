use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use thevalley_game::{NB_PLAYERS, cards, pos, deal, trick, being, star, strength};
use webgame_protocol::{GameState, PlayerInfo, ProtocolErrorKind};
use crate::{ ProtocolError };

use crate::deal::{Deal, DealSnapshot};
use crate::player::{PlayerRole, GamePlayerState};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum Phase {
    Influence,
    Act,
    Source,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum Status {
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
        let pos = self.players[&player_id].pos;
        let stars = self.stars.iter().map(|(uuid, star)|
            star.make_snapshot(player_id == *uuid, self.revealed)
        ).collect();
        GameStateSnapshot {                                          
            players,                                                 
            status: self.status,                                     
            river: self.river,                                       
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
        self.players.into_iter()
            .zip(hands.into_iter())
            .map(|((uuid, player), hand)| {
                // let last_card = source.draw();
                // hand.add(last_card);
                self.stars.insert(uuid, star::Star::new(player.pos, *hand));
                // (player.pos, last_card)
            });
        self.source = source;
        let (first, last_cards) = self.do_twilight();
        self.status = Status::Twilight(first.unwrap("what are the odds ?"), last_cards);
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
            Status::Twilight(first, _) => {
                self.status = Status::Playing(first, Phase::Influence);
            }
        }
    }

    pub fn do_twilight(&mut self) -> (Option<pos::PlayerPos>, Vec<BTreeMap<pos::PlayerPos, cards::Card>>){
        let mut drawn_cards = vec![];
        let mut first: Option<pos::PlayerPos> = None; // First player to play
        while !first.is_some() && !self.source.is_empty() {
            let last_cards: BTreeMap<pos::PlayerPos, cards::Card> = self.stars.into_iter()
                .map(|(uuid, star)| {
                    let last_card = self.source.draw();
                    star.add_to_hand(last_card);
                    (star.pos(), last_card)
                }).collect();
            first = last_cards.into_iter()
                .max_by(|(_, a), (_, b)| strength(*a).cmp(&strength(*b)))
                .map(|(pos, _)| pos);
            drawn_cards.push(last_cards);
        }

        (first, drawn_cards)
    }

    pub fn set_play(&mut self, pid: Uuid, card: cards::Card) -> Result<(), ProtocolError> {
        let pos = self.players.get(&pid).map(|p| p.pos).unwrap();
        let state = self.deal.deal_state_mut().ok_or(
            ProtocolError::new(ProtocolErrorKind::InternalError, "Unknown deal state")
        )?;
        match state.play_card(pos, card)? {
            deal::TrickResult::Nothing => (),
            deal::TrickResult::TrickOver(_winner, deal::DealResult::Nothing) => self.end_trick(),
            deal::TrickResult::TrickOver(_winner, deal::DealResult::GameOver{points: _}) => {
                self.end_last_trick();
            }
        }
        self.update_turn();
        Ok(())
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

    fn end_deal(&mut self) {
        self.turn = Turn::Interdeal;
        for player in self.players.values_mut() {
            if player.role != PlayerRole::Spectator {
                player.ready = false;
            }
        }
    }

    fn next_deal(&mut self) {
        self.first = self.first.next();
        // let auction = bid::Auction::new(self.first);
        // self.deal = Deal::Bidding(auction);
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
        match self.turn {
            Turn::Playing(pos) => Some(pos),
            // Turn::Bidding((_, pos)) => Some(pos),
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
        let pos = pos::PlayerPos::P0; // could be anything
        GameStateSnapshot {
            players: vec![],
            scores: vec![],
            turn: Turn::Pregame,
            deal: DealSnapshot {
                hand: cards::Hand::new(),
                current: pos,
                scores: [0.0;NB_PLAYERS],
                last_trick: trick::Trick::new(pos),
            }
        }
    }
}
