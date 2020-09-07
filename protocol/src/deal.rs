use serde::{Deserialize, Serialize};

use thevalley_game::{NB_PLAYERS, cards, pos, deal, trick, deal_hands};

/// Describe a single deal.
pub enum Deal {
    /// The deal is in the main playing phase
    Playing(deal::DealState),
}

impl Deal {
    // Creates a new deal, starting with an auction.
    pub fn new(first: pos::PlayerPos) -> Self {
        let (hands, river) = deal_hands();
        Deal::Playing(deal::DealState::new(pos::PlayerPos::P0, hands))
    }

    pub fn next_player(&self) -> pos::PlayerPos {
        match self {
            &Deal::Playing(ref deal) => deal.next_player(),
        }
    }

    pub fn hands(&self) -> [cards::Hand; NB_PLAYERS] {
        match self {
            &Deal::Playing(ref deal) => deal.hands(),
        }
    }

    pub fn deal_state(&self) -> Option<&deal::DealState> {
        match self {
            Deal::Playing(state) => Some(state),
        }
    }

    pub fn deal_state_mut(&mut self) -> Option<&mut deal::DealState> {
        match self {
            Deal::Playing(ref mut state) => Some(state),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DealSnapshot {
    pub hand: cards::Hand,
    pub current: pos::PlayerPos,
    pub scores: [f32; NB_PLAYERS],
    pub last_trick: trick::Trick,
    // pub tricks: Vec<trick::Trick>,
}
