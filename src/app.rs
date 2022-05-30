use crate::actions::Action;
use crate::actions::Actions;
use crate::inputs::key::Key;
use crate::io::IoEvent;
use std::time::Duration;

use log::{debug, error, warn};

#[derive(Clone)]
pub enum AppState {
    Init,
    Initialized {
        duration: Duration,
        counter_sleep: u32,
        counter_tick: u64,
    },
}

impl AppState {
    pub fn initialized() -> Self {
        let duration = Duration::from_secs(1);
        let counter_sleep = 0;
        let counter_tick = 0;
        Self::Initialized {
            duration,
            counter_sleep,
            counter_tick,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, &Self::Initialized { .. })
    }

    pub fn incr_sleep(&mut self) {
        if let Self::Initialized { counter_sleep, .. } = self {
            *counter_sleep += 1;
        }
    }

    pub fn incr_tick(&mut self) {
        if let Self::Initialized { counter_tick, .. } = self {
            *counter_tick += 1;
        }
    }

    pub fn count_sleep(&self) -> Option<u32> {
        if let Self::Initialized { counter_sleep, .. } = self {
            Some(*counter_sleep)
        } else {
            None
        }
    }

    pub fn count_tick(&self) -> Option<u64> {
        if let Self::Initialized { counter_tick, .. } = self {
            Some(*counter_tick)
        } else {
            None
        }
    }

    pub fn duration(&self) -> Option<&Duration> {
        if let Self::Initialized { duration, .. } = self {
            Some(duration)
        } else {
            None
        }
    }

    pub fn increment_delay(&mut self) {
        if let Self::Initialized { duration, .. } = self {
            // Set the duration, note that the duration is in 1s..10s
            let secs = (duration.as_secs() + 1).clamp(1, 10);
            *duration = Duration::from_secs(secs);
        }
    }

    pub fn decrement_delay(&mut self) {
        if let Self::Initialized { duration, .. } = self {
            // Set the duration, note that the duration is in 1s..10s
            let secs = (duration.as_secs() - 1).clamp(1, 10);
            *duration = Duration::from_secs(secs);
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
}

impl App {
    pub fn new(io_tx: tokio::sync::mpsc::Sender<IoEvent>) -> Self {
        let actions = vec![Action::Quit].into();
        let is_loading = false;
        let state = AppState::default();
        Self {
            io_tx,
            actions,
            is_loading,
            state,
        }
    }

    pub fn initialized(&mut self) {
        self.actions = vec![
            Action::Quit,
            Action::Sleep,
            Action::IncreaseDelay,
            Action::DecreaseDelay,
        ]
        .into();
        self.state = AppState::initialized();
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        self.state.incr_tick();
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
                Action::Sleep => {
                    if let Some(duration) = self.state.duration().cloned() {
                        self.dispatch(IoEvent::Sleep(duration)).await
                    }
                    AppReturn::Continue
                }
                Action::IncreaseDelay => {
                    self.state.increment_delay();
                    AppReturn::Continue
                }
                Action::DecreaseDelay => {
                    self.state.decrement_delay();
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

    pub fn slept(&mut self) {
        self.state.incr_sleep();
    }
}
