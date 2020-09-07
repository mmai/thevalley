//! Manage points and scores

use super::cards;

/// Returns the strength of `card`
pub fn strength(card: cards::Card) -> i32 {
    let rank = card.rank();
        match rank {
            cards::Rank::Rank1  => 1,
            cards::Rank::Rank2  => 2,
            cards::Rank::Rank3  => 3,
            cards::Rank::Rank4  => 4,
            cards::Rank::Rank5  => 5,
            cards::Rank::Rank6  => 6,
            cards::Rank::Rank7  => 7,
            cards::Rank::Rank8  => 8,
            cards::Rank::Rank9  => 9,
            cards::Rank::Rank10 => 10,
            cards::Rank::RankJ  => 1,
            cards::Rank::RankQ  => 2,
            cards::Rank::RankK  => 3,
        }
}
