use serde::{Deserialize, Serialize};

use crate::being::{Being, BeingSnapshot};
use crate::cards::{Card, Hand};
use crate::pos::PlayerPos;

/// A star
pub struct Star {
    pos: PlayerPos,
    majesty: u8,
    hand: Hand,
    beings: Vec<Being>,
}

impl Star {
    pub fn new(pos: PlayerPos, hand: Hand) -> Self {
        Star {
            pos,
            majesty: 36,
            hand,
            beings: vec![],
        }
    }

    pub fn get_hand(&self) -> Hand {
        self.hand
    }

    pub fn get_pos(&self) -> PlayerPos {
        self.pos
    }
}
                                                  
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StarSnapshot {
    pos: PlayerPos,
    majesty: u8,
    hand_count: u8,
    beings: Vec<BeingSnapshot>,
}
                                                  
impl Star {

    pub fn add_to_hand(&mut self, card: Card){
        self.hand.add(card);
    }

    pub fn pos(&self) -> PlayerPos {
        self.pos
    }

    pub fn make_snapshot(&self, with_hand: bool, revealed: &Vec<Card>) -> StarSnapshot{
        let hand = if with_hand { Some(self.hand) } else { None };
        let beings = self.beings.iter().map(|b| b.make_snapshot(revealed)).collect();
        StarSnapshot {
            pos: self.pos,
            majesty: self.majesty,
            hand_count: hand.iter().len() as u8,
            beings,
        }
    }
}                                     
                                                  
