use serde::{Deserialize, Serialize};

use crate::message::ProtocolError;
use webgame_protocol::ProtocolErrorKind;
use thevalley_game::{cards, deal};

impl From<deal::PlayError> for ProtocolError {
    fn from(error: deal::PlayError) -> Self {
        ProtocolError::new(
            ProtocolErrorKind::BadState,
            format!("play: {}", error)
       )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum GamePlayCommand {
    Play(PlayCommand),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayCommand {
    pub card: cards::Card,
}
