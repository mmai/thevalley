use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::cards::{Card, Suit};

/// A being
pub struct Being {
    face: Card,
    resources: BTreeMap<Suit, Card>,
    loved_one: Option<Card>,
}

/// A being
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BeingSnapshot {
    pub face: Card,
    pub resources: BTreeMap<Suit, Option<Card>>,
    // pub loved_one: Option<Card>,
}

impl Being {
    pub fn make_snapshot(&self, revealed: &Vec<Card>) -> BeingSnapshot {
        let resources = self.resources.clone().into_iter().map(|(s, c)| {
            if revealed.contains(&c) { 
                (s, Some(c)) 
            } else { 
                (s, None) 
            }
        }).collect();
        BeingSnapshot {
            face: self.face,
            resources,
        }

    }

  pub fn heart(&self) -> Option<Card> {
      self.get_resource(&Suit::Heart)
  }

  pub fn weapon(&self) -> Option<Card> {
      self.get_resource(&Suit::Spade)
  }

  pub fn mind(&self) -> Option<Card> {
      self.get_resource(&Suit::Diamond)
  }
                                                  
  pub fn power(&self) -> Option<Card> {           
      self.get_resource(&Suit::Club)              
  }                                               
                                                  
  fn get_resource(&self, suit: &Suit) -> Option<Card> {
      self.resources.get(suit).map(|res| *res)    
  }                              
                                
}                                
                                 
                                 
                                 
