use std::rc::Rc;
use std::time::Duration;
use std::f32;
use im_rc::Vector;
use uuid::Uuid;
use yew::agent::Bridged;
use yew::services::{IntervalService, Task};
use yew::{
    html, Bridge, Component, ComponentLink, Html, Properties,
    ShouldRender,
};
use tr::tr;

use crate::api::Api;
use crate::components::chat_box::{ChatBox, ChatLine, ChatLineData};
use crate::components::scores::Scores;
use crate::gprotocol::{GameInfo, PlayerInfo, SendTextCommand};
use crate::protocol::{
    Command, GamePlayerState, GameStateSnapshot, Message, PlayerAction,
    GamePlayCommand,
    PlayCommand,
    PlayEvent,
    Status,
    Phase,
};
use thevalley_game::cards;
use crate::utils::format_join_code;
use crate::sound_player::SoundPlayer;

#[derive(Clone, Properties)]
pub struct Props {
    pub player_info: PlayerInfo,
    pub game_info: GameInfo,
}

pub struct GamePage {
    #[allow(dead_code)]
    keepalive_job: Box<dyn Task>,
    link: ComponentLink<GamePage>,
    api: Box<dyn Bridge<Api>>,
    game_info: GameInfo,
    player_info: PlayerInfo,
    game_state: Rc<GameStateSnapshot>,
    chat_log: Vector<Rc<ChatLine>>,
    hand: cards::Hand,
    is_waiting: bool,
    sound_player: SoundPlayer,
    error: Option<String>,
}

pub enum Msg {
    Ping,
    Disconnect,
    MarkReady,
    Continue,
    CloseError,
    Play(cards::Card),
    SetChatLine(String),
    AddToHand(cards::Card),
    ServerMessage(Message),
}

impl GamePage {
    pub fn add_chat_message(&mut self, player_id: Uuid, data: ChatLineData) {
        let nickname = self
            .game_state
            .players
            .iter()
            .find(|x| x.player.id == player_id)
            .map(|x| x.player.nickname.as_str())
            .unwrap_or("anonymous")
            .to_string();
        self.chat_log
            .push_back(Rc::new(ChatLine { nickname, data }));
        while self.chat_log.len() > 100 {
            self.chat_log.pop_front();
        }
    }

    pub fn my_state(&self) -> &GamePlayerState {
        self.game_state
            .players
            .iter()
            .find(|state| state.player.id == self.player_info.id)
            .unwrap()
    }

    // XXX works only for two players game.
    pub fn opponent_state(&self) -> &GamePlayerState {
        self.game_state
            .players
            .iter()
            .find(|state| state.player.id != self.player_info.id)
            .unwrap()
    }
}

impl Component for GamePage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        // Ping server every 50s in order to keep alive the websocket 
        let keepalive = IntervalService::spawn(
            Duration::from_secs(50), 
            link.callback(|_| Msg::Ping).into()
            );

        let on_server_message = link.callback(Msg::ServerMessage);
        let api = Api::bridge(on_server_message);
        let sound_paths = vec![
            ("chat".into(), "sounds/misc_menu.ogg"),
            ("card".into(), "sounds/cardPlace4.ogg"),
            ("error".into(), "sounds/negative_2.ogg"),
        ].into_iter().collect();

        GamePage {
            keepalive_job: Box::new(keepalive),
            link,
            api,
            game_info: props.game_info,
            chat_log: Vector::unit(Rc::new(ChatLine {
                nickname: props.player_info.nickname.clone(),
                data: ChatLineData::Connected,
            })),
            game_state: Rc::new(GameStateSnapshot::default()),
            player_info: props.player_info,
            hand: cards::Hand::new(),
            is_waiting: false,
            sound_player: SoundPlayer::new(sound_paths),
            error: None,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ServerMessage(message) => match message {
                Message::Chat(msg) => {
                    self.sound_player.play("chat".into());
                    self.add_chat_message(msg.player_id, ChatLineData::Text(msg.text));
                }
                Message::PlayEvent(evt) => {
                    self.sound_player.play("card".into());
                    let PlayEvent::Play(uuid, card) = evt;
                    self.add_chat_message(uuid, ChatLineData::Text(format!("play: {}", card.to_string())));
                    unsafe {log!("play event {:?}", evt)};
                }
                Message::Error(e) => {
                    self.is_waiting = false;
                    self.error = Some(e.message().into());
                    self.sound_player.play("error".into());
                    unsafe {log!("error from server {:?}", e)};
                }
                Message::PlayerConnected(state) => {
                    let player_id = state.player.id;
                    let game_state = Rc::make_mut(&mut self.game_state);
                    game_state.players.push(state);
                    self.add_chat_message(player_id, ChatLineData::Connected);
                }
                Message::PlayerDisconnected(msg) => {
                    self.add_chat_message(msg.player_id, ChatLineData::Disconnected);
                    let game_state = Rc::make_mut(&mut self.game_state);
                    game_state.players.retain(|x| x.player.id != msg.player_id);
                }
                Message::GameStateSnapshot(snapshot) => {
                    self.is_waiting = false;
                    self.game_state = Rc::new(snapshot);
                    self.hand = self.game_state.hand;
                }
                _ => {}
            },
            Msg::Ping => {
                self.api.send(Command::Ping);
                // log!("ping ?");
            }
            Msg::SetChatLine(text) => {
                self.api.send(Command::SendText(SendTextCommand { text }));
            }
            Msg::CloseError => {
                self.error = None;
            }
            Msg::Continue => {
                self.is_waiting = true;
                self.api.send(Command::Continue);
            }
            Msg::MarkReady => {
                self.is_waiting = true;
                self.api.send(Command::MarkReady);
            }
            Msg::Disconnect => {
                self.api.send(Command::LeaveGame);
            }
            Msg::AddToHand(card) => {
                self.hand.add(card);
            },
            Msg::Play(card) => {
                self.is_waiting = true;
                self.api.send(Command::GamePlay(GamePlayCommand::Play(PlayCommand { card })));
            }
        }
        true
    }

    fn view(&self) -> Html {
        if self.game_state.players.is_empty() {
            return html! {};
        }

        let my_state = self.my_state();

        // display players in order of playing starting from the current player
        let mut others_before = vec![];
        let mut others = vec![];
        let mypos = my_state.pos.to_n();

        // let mut positioned = Vec::from_iter(self.game_state.players.clone());
        // positioned.sort_by(|a, b| a.pos.to_n().cmp(&b.pos.to_n()));
        // for pstate in positioned.iter() {
        for pstate in self.game_state.players.iter() {
            let pos = pstate.pos.to_n();
            if pos < mypos {
                others_before.push(pstate.clone());
            } else if mypos < pos{
               others.push(pstate.clone());
            }
        }

        // log!("others: {:?} others_before: {:?}", others, others_before);
        others.append(&mut others_before);

        let mut game_classes = vec!["game"];
        if self.is_waiting {
            game_classes.push("waiting");
        }

        let is_my_turn = self.game_state.get_playing_pos() == Some(self.my_state().pos);
        // let is_my_turn = self.game_state.turn.has_player_pos() && self.game_state.deal.current == self.my_state().pos;
        let mut actions_classes = vec!["actions"];
        if is_my_turn {
            actions_classes.push("current-player");
        }

        let message_content: Option<Html> = match self.game_state.status {
               Status::Endgame => 
                   if !self.my_state().ready  { 
                       let players: Vec<String> = self.game_state.players.iter().map(|pl| pl.player.nickname.clone()).collect();

                       Some(html! {
                     <div>
                     {{ "Fini" }}
                     </div>
                   })} else { None },
              _ => None
        };

        let player = self.game_state.current_player_name();
        let status_info = match self.game_state.status {
            Status::Pregame => tr!("pre-game"),
            Status::Twilight(_, _) => tr!("{0} starting", player),
            Status::Playing(_, Phase::Influence) => tr!("{0} influence", player),
            Status::Playing(_, Phase::Act) => tr!("{0} act", player),
            Status::Playing(_, Phase::Source) => tr!("{0} source", player),
            Status::Endgame => tr!("end"),
        };

        html! {
    <div class=game_classes>
      <header>
        <p class="turn-info">{status_info}</p>
      </header>

        { if let Some(error) = &self.error  { 
            let error_str = match error.as_str() {
            "play: invalid turn order" => tr!("invalid turn order"),
            "play: you can only play cards you have" => tr!("you can only play cards you have" ),
            "play: no trick has been played yet" => tr!("no trick has been played yet" ),
            _ => error.to_string()
            };
            html! {
          <div class="notify-wrapper">
            <div class="error notify">
                <div>
                { error_str } 
                </div>
                <div class="toolbar">
                    <button class="btn-error" onclick=self.link.callback(|_| Msg::CloseError)>{"Ok"}</button>
                </div>
              </div>
            </div>
        }} else { html! {} }}

        { if let Some(message) = message_content  { html! {
          <div class="notify-wrapper">
            <div class="notify wrapper">
                { message }

                <div class="toolbar">
                    <button class="primary" onclick=self.link.callback(|_| Msg::Continue)>{"Ok"}</button>
                </div>
            </div>
        </div>
        }} else { html! {} }}

        <section class=actions_classes>
            {match &self.game_state.status {
               Status::Pregame => html! {
                <div class="wrapper">
                    <div class="toolbar">
                    {if !self.my_state().ready  {
                        html! {<button class="primary" onclick=self.link.callback(|_| Msg::MarkReady)>{ tr!("Ready!")}</button>}
                    } else {
                        html! {}
                    }}
                        <button class="cancel" onclick=self.link.callback(|_| Msg::Disconnect)>{ tr!("Disconnect") }</button>
                    </div>
                    <h1>{{ tr!("join code:") }} <strong>{format!(" {}", format_join_code(&self.game_info.join_code))}</strong></h1>
                 </div>
                },
               Status::Twilight(first, cards) => {
                   let opponent_state = self.opponent_state();
                   let my_cards: Vec<&cards::Card> = cards.iter().map(|draw| draw.get(&my_state.pos).unwrap()).collect();
                   let opponent_cards: Vec<&cards::Card> = cards.iter().map(|draw| draw.get(&opponent_state.pos).unwrap()).collect();
                   let first_name = if (first == &my_state.pos) { 
                       String::from("you")
                   } else {
                       opponent_state.player.nickname.clone()
                   };
               html! {
                   <div id="twilight-cards">
                       <div class="hand">
                         { for opponent_cards.iter().map(|card| {
                               let style =format!("--bg-image: url('cards/{}-{}.svg')", &card.rank().to_string(), &card.suit().to_safe_string());
                               html! { <div class="card" style={style}></div> }
                           })
                         }
                       </div>
                       <div class="hand">
                        { for my_cards.iter().map(|card| {
                               let style =format!("--bg-image: url('cards/{}-{}.svg')", &card.rank().to_string(), &card.suit().to_safe_string());
                               html! { <div class="card" style={style}></div> }
                           })
                        }
                     </div>

                     <div class="notify-wrapper">
                         <div class="notify wrapper">
                             {format!(" {} will start", first_name)}
                             <div class="toolbar">
                                 <button class="primary" onclick=self.link.callback(|_| Msg::Continue)>{"Ok"}</button>
                             </div>
                         </div>
                     </div>


                 </div>
               }},
                _ => 
                    html! {
                        <div>
                            <div>{{ "Source" }}{format!(" {}", &self.game_state.source_count)}</div>
                            <div>{{ "River" }}</div>
                            <div>{{ "my beings" }}</div>
                        </div>
                    }
             }}
        </section>

        <section class="hand">
        { if self.game_state.status != Status::Pregame {
           let hidden_cards: Vec<&cards::Card> = 
               if let Status::Twilight(first, cards) = &self.game_state.status {
                   cards.iter().map(|draw| draw.get(&my_state.pos).unwrap()).collect()
               } else { vec![] };
           html! {
              for self.hand.list().iter().filter(|card| !hidden_cards.contains(card)).map(|card| {
                let style =format!("--bg-image: url('cards/{}-{}.svg')", &card.rank().to_string(), &card.suit().to_safe_string());
                let clicked = card.clone();
                html! {
                    <div class="card" style={style} 
                    onclick=self.link.callback(move |_| Msg::Play(clicked) ) >
                    </div>
                }
            })
        }} else {
            html!{}
        }}
        </section>

        <section class="bottom">
        { if self.game_state.status != Status::Pregame {
           html! {
            {{ self.game_state.star().get_majesty() }}
        }} else {
            html!{}
        }}
        </section>

        <ChatBox log=self.chat_log.clone()
                 on_send_chat=self.link.callback(|text| Msg::SetChatLine(text))
        />

    </div>

        }
    }
}
