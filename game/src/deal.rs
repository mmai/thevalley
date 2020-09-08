//! Module for the card deal, after auctions are complete.
use std::fmt;

use super::cards;
use super::pos;
use super::trick;

/// Describes the state of a deal, ready to play a card.
#[derive(Clone)]
pub struct DealState {
    players: [cards::Hand; super::NB_PLAYERS],
    current: pos::PlayerPos,
    points: [f32; super::NB_PLAYERS],
    tricks: Vec<trick::Trick>,
}

/// Result of a deal.
#[derive(PartialEq, Debug)]
pub enum DealResult {
    /// The deal is still playing
    Nothing,

    /// The deal is over
    GameOver {
        /// Worth of won tricks
        points: [f32; super::NB_PLAYERS],
    },
}

/// Result of a trick
#[derive(PartialEq, Debug)]
pub enum TrickResult {
    Nothing,
    TrickOver(pos::PlayerPos, DealResult),
}

/// Error that can occur during play
#[derive(PartialEq, Debug)]
pub enum PlayError {
    /// A player tried to act before his turn
    TurnError,
    /// A player tried to play a card he doesn't have
    CardMissing,
    /// A player tried to play the wrong suit, while he still have some
    IncorrectSuit,
    /// A player tried to play the wrong suit, while he still have trumps
    NoLastTrick,
}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PlayError::TurnError => write!(f, "invalid turn order"),
            PlayError::CardMissing => write!(f, "you can only play cards you have"),
            PlayError::IncorrectSuit => write!(f, "wrong suit played"),
            PlayError::NoLastTrick => write!(f, "no trick has been played yet"),
        }
    }
}

impl DealState {
    /// Creates a new DealState, with the given cards, first player and contract.
    pub fn new(first: pos::PlayerPos, hands: [cards::Hand; super::NB_PLAYERS]) -> Self {
        DealState {
            players: hands,
            current: first,
            tricks: vec![trick::Trick::new(first)],
            points: [0.0; 2],
        }
    }

    /// Try to play a card
    pub fn play_card(
        &mut self,
        player: pos::PlayerPos,
        card: cards::Card,
    ) -> Result<TrickResult, PlayError> {
        if self.current != player {
            return Err(PlayError::TurnError);
        }

        let is_first_trick = self.tricks.len() == 1;

        // Is that a valid move?
        can_play(
            player,
            card,
            self.players[player as usize],
            self.current_trick(),
            is_first_trick,
        )?;

        // Play the card
        let trick_over = self.current_trick_mut().play_card(player, card);

        // Remove card from player hand
        self.players[player as usize].remove(card);

        // Is the trick over?
        let result = if trick_over {
            let winner = self.current_trick().winner;

            if self.tricks.len() == super::DEAL_SIZE {
                // TODO petit au bout ? -> maj annonce
            } else {
                self.tricks.push(trick::Trick::new(winner));
            }
            self.current = winner;
            TrickResult::TrickOver(winner, self.get_deal_result())
        } else {
            self.current = self.current.next();
            TrickResult::Nothing
        };

        Ok(result)
    }

    /// Returns the player expected to play next.
    pub fn next_player(&self) -> pos::PlayerPos {
        self.current
    }

    pub fn get_deal_result(&self) -> DealResult {
        DealResult::Nothing
    }

    /// Returns the cards of all players
    pub fn hands(&self) -> [cards::Hand; super::NB_PLAYERS] {
        self.players
    }

    pub fn is_over(&self) -> bool {
        self.tricks.len() == super::DEAL_SIZE && !self.tricks[super::DEAL_SIZE -1].cards.iter().any(|&c| c.is_none())
    }

    /// Return the last trick, if possible
    pub fn last_trick(&self) -> Result<&trick::Trick, PlayError> {
        if self.tricks.len() == 1 {
            Err(PlayError::NoLastTrick)
        } else {
            let i = self.tricks.len() - 2;
            Ok(&self.tricks[i])
        }
    }

    /// Returns the current trick.
    pub fn current_trick(&self) -> &trick::Trick {
        let i = self.tricks.len() - 1;
        &self.tricks[i]
    }

    fn current_trick_mut(&mut self) -> &mut trick::Trick {
        let i = self.tricks.len() - 1;
        &mut self.tricks[i]
    }
}

/// Returns `true` if the move appear legal.
pub fn can_play(
    _p: pos::PlayerPos,
    card: cards::Card,
    hand: cards::Hand,
    _trick: &trick::Trick,
    _is_first_trick:bool,
) -> Result<(), PlayError> {
    // First, we need the card to be able to play
    if !hand.has(card) {
        return Err(PlayError::CardMissing);
    }

    Ok(())
}
