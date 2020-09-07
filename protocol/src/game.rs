use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use thevalley_game::{NB_PLAYERS, cards, pos, deal, trick};
use webgame_protocol::{GameState, PlayerInfo, ProtocolErrorKind};
use crate::{ ProtocolError };

use crate::turn::Turn;
use crate::deal::{Deal, DealSnapshot};
use crate::player::{PlayerRole, GamePlayerState};

pub struct ValleyGameState {
    players: BTreeMap<Uuid, GamePlayerState>,
    turn: Turn,
    deal: Deal,
    first: pos::PlayerPos,
    scores: Vec<[f32; NB_PLAYERS]>,
}

impl Default for ValleyGameState {
    fn default() -> ValleyGameState {
        ValleyGameState {
            players: BTreeMap::new(),
            turn: Turn::Pregame,
            deal: Deal::new(pos::PlayerPos::P0),
            first: pos::PlayerPos::P0,
            scores: vec![],
        }
    }
}

impl GameState< GamePlayerState, GameStateSnapshot> for ValleyGameState {
    type PlayerPos = pos::PlayerPos;
    type PlayerRole = PlayerRole;

    fn is_joinable(&self) -> bool {
        self.turn == Turn::Pregame
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
        let scores = [0.0; NB_PLAYERS];
        let deal = match self.deal.deal_state() {
            Some(state) => { // In Playing phase
                let last_trick = if self.turn == Turn::Intertrick && !self.was_last_trick() {
                    // intertrick : there is at least a trick done
                    state.last_trick().unwrap().clone()
                } else {
                    state.current_trick().clone()
                };
                // log::debug!("trick {:?}", last_trick.cards);
                DealSnapshot {
                    hand: state.hands()[pos as usize],
                    current: state.next_player(),
                    scores,
                    last_trick,
                }
            },
            None => DealSnapshot { // In bidding phase
                hand: self.deal.hands()[pos as usize],
                current: self.deal.next_player(),
                scores: [0.0;NB_PLAYERS],
                last_trick: trick::Trick::default(),
            }
        };
        GameStateSnapshot {
            players,
            scores: self.scores.clone(),
            turn: self.turn,
            deal
        }
    }

    fn set_player_ready(&mut self, player_id: Uuid){
        let turn = self.turn.clone();
        if let Some(player_state) = self.players.get_mut(&player_id) {
            player_state.ready = true;
            if turn == Turn::Intertrick {
                self.update_turn();
            } else {
                player_state.role = PlayerRole::PreDeal;

                // Check if we start the next deal
                let mut count = 0;
                for player in self.players.values() {
                    if player.role == PlayerRole::PreDeal {
                        count = count + 1;
                    }
                }
                if count == NB_PLAYERS {
                    if self.turn == Turn::Interdeal { // ongoing game
                        self.update_turn();
                    } else { // new game
                        // self.turn = Turn::Bidding((bid::AuctionState::Bidding, pos::PlayerPos::P0));
                    }
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

impl ValleyGameState {
    pub fn get_turn(&self) -> Turn {
        self.turn
    }

    fn position_taken(&self, position: pos::PlayerPos) -> bool {
        self.player_by_pos(position) != None
    }

    pub fn players_ready(&self) -> bool {
        !(self.players.iter().find(|(_, player)| player.ready == false) != None)
    }

    pub fn update_turn(&mut self){
        self.turn = if !self.players_ready() {
            Turn::Intertrick
        } else if self.was_last_trick() {
            self.end_deal();
            Turn::Interdeal
        } else {
            if self.turn == Turn::Interdeal {
                self.next_deal();
            }
            Turn::from_deal(&self.deal)
        }
    }

    fn was_last_trick(&self) -> bool {
        let p0 = self.player_by_pos(pos::PlayerPos::P0).unwrap();
        self.turn == Turn::Intertrick && p0.role == PlayerRole::Unknown
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
    pub turn: Turn,
    pub deal: DealSnapshot,
    pub scores: Vec<[f32; NB_PLAYERS]>,
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
