use std::fmt;
use serde::{Deserialize, Serialize};

use thevalley_game::pos;
use crate::deal::Deal;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Turn {
    Pregame,
    Intertrick,
    Interdeal,
    Playing(pos::PlayerPos),
    Endgame,
}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strpos;
        write!(
            f,
            "{}",
            match *self {
                Turn::Pregame => "pre-game",
                Turn::Intertrick => "inter trick",
                Turn::Interdeal => "inter deal",
                Turn::Playing(_pos) => {
                    strpos = format!("playing");
                    // strpos = format!("{:?} to play", pos);
                    &strpos
                }
                Turn::Endgame => "end",
            }
        )
    }
}

impl Turn {
    pub fn has_player_pos(&self) -> bool {
        match self {
            Self::Pregame => false,
            Self::Interdeal => false,
            Self::Endgame => false,
            _ => true
        }
    }

    pub fn from_deal(deal: &Deal) -> Self {
        match deal {
            Deal::Playing(deal_state) => {
                Self::Playing(deal_state.next_player())
            },
        }
    }
}

