use std::sync::Arc;

use eyre::Result;
use log::{error, info};

use super::IoEvent;
use crate::app::App;

pub struct IoAsyncHandler {
    app: Arc<tokio::sync::Mutex<App>>,
}

impl IoAsyncHandler {
    pub fn new(app: Arc<tokio::sync::Mutex<App>>) -> Self {
        Self { app }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        let result = match io_event {
            IoEvent::Initialize => self.do_initialize().await,
            IoEvent::SongIsOver => self.play_next_song().await,
        };

        if let Err(err) = result {
            error!("Oops, something wrong happen {:?}", err);
        }

        let mut app = self.app.lock().await;
        app.loaded();
    }

    async fn do_initialize(&mut self) -> Result<()> {
        info!("Initialize the application");
        let mut app = self.app.lock().await;
        app.initialized();
        info!("Application initialized");

        Ok(())
    }

    async fn play_next_song(&self) -> Result<()> {
        info!(" Switching to the next song");

        let mut app = self.app.lock().await;
        app.song_switched();
        info!("The song is switched");
        Ok(())
    }
}
