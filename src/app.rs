use crate::actions::Action;
use crate::actions::Actions;
use crate::inputs::key::Key;

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    actions: Actions,
}

impl App {
    pub fn new() -> Self {
        let actions = vec![Action::Quit].into();
        Self { actions }
    }

    pub fn do_action(&mut self, key: Key) -> AppReturn {
        if let Some(action) = self.actions.find(key) {
            match action {
                Action::Quit => AppReturn::Exit,
            }
        } else {
            AppReturn::Continue
        }
    }

    pub fn update_on_tick(&mut self) -> AppReturn {
        AppReturn::Continue
    }
}
