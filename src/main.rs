mod actions;
mod app;
mod inputs;
mod io;
mod ui;

use app::App;
use app::AppReturn;

use inputs::events::Events;
use inputs::InputEvent;

use io::handler::IoAsyncHandler;
use io::IoEvent;

use eyre::Result;

use log::LevelFilter;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn start_ui(app: &Arc<tokio::sync::Mutex<App>>) -> Result<(), Box<dyn Error>> {
    let stdout = std::io::stdout();

    crossterm::terminal::enable_raw_mode();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    {
        let mut app = app.lock().await;
        app.dispatch(IoEvent::Initialize).await;
    }

    let tick_rate = Duration::from_millis(200);
    let mut events = Events::new(tick_rate);
    loop {
        let mut app = app.lock().await;
        terminal.draw(|rect| ui::draw(rect, &app))?;

        let result = match events.next().await {
            InputEvent::Input(key) => app.do_action(key).await,
            InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            events.close();
            break;
        }
    }

    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tui_logger::init_logger(LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Debug);

    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<IoEvent>(100);

    let app = Arc::new(tokio::sync::Mutex::new(App::new(sync_io_tx.clone())));
    let app_ui = Arc::clone(&app);

    tokio::spawn(async move {
        let mut handler = IoAsyncHandler::new(app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_io_event(io_event).await;
        }
    });

    start_ui(&app_ui).await?;
    Ok(())
}
