use serde::{Deserialize, Serialize};

use crate::being::{Being, BeingSnapshot};
use crate::cards::Hand;
use crate::pos::PlayerPos;

/// A star
pub struct Star {
    pos: PlayerPos,
    majesty: i32,
    hand: Hand,
    beings: Vec<Being>,
}
                                                  
impl Star {                                       
}                                                 
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StarSnapshot {
    pos: PlayerPos,
    majesty: i32,
    hand: Option<Hand>,
    beings: Vec<BeingSnapshot>,
}
                                                  
                                                  
                                                  
                                                  
                                                  



                        
                        
                        
                        
                        
                        
                        
