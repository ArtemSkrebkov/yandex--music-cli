mod actions;
mod app;
mod inputs;
mod ui;

use app::App;
use app::AppReturn;

use inputs::events::Events;
use inputs::InputEvent;

use std::cell::RefCell;
use std::error::Error;
use std::io;
use std::rc::Rc;
use std::time::Duration;
use tui::{backend::CrosstermBackend, Terminal};

pub fn start_ui(app: Rc<RefCell<App>>) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout();

    crossterm::terminal::enable_raw_mode();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let tick_rate = Duration::from_millis(200);
    let events = Events::new(tick_rate);

    loop {
        let mut app = app.borrow_mut();
        terminal.draw(|rect| ui::draw(rect, &app))?;

        let result = match events.next()? {
            InputEvent::Input(key) => app.do_action(key),
            InputEvent::Tick => app.update_on_tick(),
        };

        if result == AppReturn::Exit {
            break;
        }
    }

    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode();

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = Rc::new(RefCell::new(App::new()));
    start_ui(app)?;
    Ok(())
}
