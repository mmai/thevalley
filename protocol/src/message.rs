use serde::{Deserialize, Serialize};

use webgame_protocol::{ProtocolError as GenericProtocolError, ProtocolErrorKind, Message as GenericMessage, Command as GenericCommand};

use crate::player::{PlayerRole, GamePlayerState};
use crate::game::{GameStateSnapshot, PlayEvent};
use crate::game_messages::GamePlayCommand;


impl From<ProtocolError> for GenericProtocolError {
    fn from(error: ProtocolError) -> Self {
        GenericProtocolError::new(
            error.kind,
            error.message      
       )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolError {
    kind: ProtocolErrorKind,
    message: String,
}

impl ProtocolError {
    pub fn new<S: Into<String>>(kind: ProtocolErrorKind, s: S) -> ProtocolError {
        ProtocolError {
            kind,
            message: s.into(),
        }
    }

    pub fn kind(&self) -> ProtocolErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetPlayerRoleCommand {
    pub role: PlayerRole,
}

pub type Message = GenericMessage<GamePlayerState, GameStateSnapshot, PlayEvent>;
pub type Command = GenericCommand<GamePlayCommand, SetPlayerRoleCommand, GameStateSnapshot>;
