use crate::actions::Action;
use crate::actions::Actions;
use crate::inputs::key::Key;
use crate::io::IoEvent;
use std::time::Duration;

use log::{debug, error, warn};
use yandex_rust_music::{Client, Player, Status};

#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        duration: Duration,
        total_duration: Duration,
    },
}

impl AppState {
    pub fn initialized(total_duration: &Duration) -> Self {
        let duration = Duration::from_secs(0);
        Self::Initialized {
            duration,
            total_duration: total_duration.clone(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn update_duration(&mut self, current_duration: Duration) {
        if let Self::Initialized { duration, .. } = self {
            *duration = current_duration;
        }
    }

    pub fn duration(&self) -> Option<&Duration> {
        if let Self::Initialized { duration, .. } = self {
            Some(duration)
        } else {
            None
        }
    }

    pub fn total_duration(&self) -> Option<&Duration> {
        if let Self::Initialized { total_duration, .. } = self {
            Some(total_duration)
        } else {
            None
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    io_tx: tokio::sync::mpsc::Sender<IoEvent>,
    actions: Actions,
    is_loading: bool,
    state: AppState,
    client: Client,
    player: Player,
    status: Status,
}

impl App {
    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let actions = vec![Action::Quit].into();
        let is_loading = false;
        let state = AppState::default();
        let client = Client::new("AQAAAAA59C-DAAG8Xn4u-YGNfkkqnBG_DcwEnjM");
        // TODO: Player::default
        let player = Player::new();
        Self {
            io_tx,
            actions,
            is_loading,
            state,
            client,
            player,
            status: Status::Paused(Duration::from_secs(0)),
        }
    }

    pub fn initialized(&mut self) {
        self.actions = vec![Action::Quit, Action::PlaySound, Action::PauseSound].into();
        let track = self.client.get_random_track();
        let track_path = track.download();
        self.player.append(&track_path);
        let total_duration = track.total_duration().unwrap();
        debug!("Added song {}...", track_path);
        self.state = AppState::initialized(&total_duration);
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        self.state.update_duration(self.status.elapsed());
        AppReturn::Continue
    }

    pub async fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;
        if let Err(e) = self.io_tx.send(action).await {
            self.is_loading = false;
            error!("Error from dispatch {}", e);
        }
    }

    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        if let Some(action) = self.actions.find(key) {
            debug!("Run action [{:?}]", action);
            match action {
                Action::Quit => AppReturn::Exit,
                Action::PlaySound => {
                    self.player.play();
                    self.status.play();
                    AppReturn::Continue
                }
                Action::PauseSound => {
                    self.player.pause();
                    self.status.pause();
                    AppReturn::Continue
                }
            }
        } else {
            warn!("No action associated to {}", key);
            AppReturn::Continue
        }
    }

    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn loaded(&mut self) {
        self.is_loading = false;
    }
}
