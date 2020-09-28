use serde::{Deserialize, Serialize};

use thevalley_game::cards;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum GamePlayCommand {
    Play(PlayCommand),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayCommand {
    pub card: cards::Card,
}
