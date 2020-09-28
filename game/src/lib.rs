#[macro_use]

#[cfg(feature = "use_bench")]
extern crate test;

pub mod cards;
pub mod points;
pub mod pos;
pub mod trick;
pub mod being;
pub mod star;

pub const NB_PLAYERS:usize = 2;
const DEAL_SIZE:usize = 10 ;

// Expose the module or their content directly? Still unsure.

// pub use bid::*;
// pub use cards::*;
// pub use deal::*;
pub use points::*;
// pub use pos::*;
// pub use trick::*;

/// Deals cards to 2 players randomly.
pub fn deal_hands() -> ([cards::Hand; NB_PLAYERS], cards::Deck) {
    let mut hands = [cards::Hand::new(); NB_PLAYERS];
    let mut source = cards::Deck::new();
    source.shuffle();    
                        
    source.deal_each(&mut hands, 9);

    (hands, source)
}

/// Deal cards deterministically.
pub fn deal_seeded_hands(seed: [u8; 32]) -> ([cards::Hand; NB_PLAYERS], cards::Deck) {
    let mut hands = [cards::Hand::new(); NB_PLAYERS];
    let mut source = cards::Deck::new();
    source.shuffle_seeded(seed);

    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);
    source.deal_each(&mut hands, 1);

    (hands, source)
}

#[test]
fn test_deals() {
    let (hands, source) = deal_hands();
    assert!(source.len() == 36);

    let mut count = [0; 54];

    for card in source.iter() {
        count[card.id() as usize] += 1;
    }
    for hand in hands.iter() {
        assert!(hand.size() == DEAL_SIZE);
        for card in hand.list().iter() {
            count[card.id() as usize] += 1;
        }
    }

    for c in count.iter() {
        assert!(*c == 1);
    }

}

