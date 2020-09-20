use serde::{Deserialize, Serialize};

use thevalley_game::pos;
use webgame_protocol::{PlayerInfo, PlayerState};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRole {
    Unknown,
    PreDeal,
    Spectator,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Play,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GamePlayerState {
    pub player: PlayerInfo,
    pub pos: pos::PlayerPos,
    pub role: PlayerRole,
    pub ready: bool,
}

impl PlayerState for GamePlayerState {
    fn player(self) -> PlayerInfo {
        self.player
    }
}

impl GamePlayerState {
}

