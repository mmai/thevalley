use uuid::Uuid;
use std::sync::Arc;

use crate::webgame_server::universe::Universe;
use crate::webgame_server::game::Game;
use crate::gprotocol::GameState;

use crate::gprotocol::{ 
    Message, ChatMessage,
    ProtocolError, ProtocolErrorKind 
};

use crate::protocol::{ 
    GamePlayCommand, 
    SetPlayerRoleCommand, 
    PlayEvent,
    PlayCommand,
    ValleyGameState,
    GamePlayerState,
    GameStateSnapshot
};

//see https://users.rust-lang.org/t/how-to-store-async-function-pointer/38343/4
type DynFut<T> = ::std::pin::Pin<Box<dyn Send + ::std::future::Future<Output = T>>>;

pub fn on_gameplay(
    universe: Arc<Universe<ValleyGameState, GamePlayerState, GameStateSnapshot, PlayEvent>>,
    user_id: Uuid,
    cmd: GamePlayCommand,
) -> DynFut<Result<(), ProtocolError>> {
    Box::pin(async move {
        if let Some(game) = universe.get_user_game(user_id).await {
            match cmd {
                GamePlayCommand::Play(cmd) => on_player_play(game, user_id, cmd).await,
            }                        
        } else {
            Err(ProtocolError::new(
                    ProtocolErrorKind::BadState,
                    "not in a game",
            ))
        }
    })
}                                

pub fn on_player_set_role(
    universe: Arc<Universe<ValleyGameState, GamePlayerState, GameStateSnapshot, PlayEvent>>,
    user_id: Uuid,
    cmd: SetPlayerRoleCommand,
) -> DynFut<Result<(), ProtocolError>> {
    Box::pin(async move {
        if let Some(game) = universe.get_user_game(user_id).await {
            if !game.is_joinable().await {
                return Err(ProtocolError::new(
                        ProtocolErrorKind::BadState,
                        "cannot set role because game is not not joinable",
                ));
            }

            let game_state = game.state_handle();
            let mut game_state = game_state.lock().await;
            game_state.set_player_role(user_id, cmd.role);

            game.set_player_not_ready(user_id).await;
            game.broadcast_state().await;
            Ok(())
        } else {
            Err(ProtocolError::new(
                    ProtocolErrorKind::BadState,
                    "not in a game",
            ))
        }
    })
}

pub async fn on_player_play(
    game: Arc<Game<ValleyGameState, GamePlayerState, GameStateSnapshot, PlayEvent>>,
    player_id: Uuid,
    cmd: PlayCommand,
) -> Result<(), ProtocolError> {
        let game_state = game.state_handle();
        let mut game_state = game_state.lock().await;
        if let Err(e) = game_state.set_play(player_id, cmd.card) {
            game.send(player_id, &Message::Error(e.into())).await;
        } else {
            game.broadcast(&Message::PlayEvent(PlayEvent::Play ( player_id, cmd.card )))
            .await;
            game.broadcast_state().await;
        }
        Ok(())
}
