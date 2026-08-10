use anyhow::Result;
use cineplanet_cli::{
    app::{Action, App, Effect},
    demo, live, settings, ui,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

fn main() -> Result<()> {
    let preferences = settings::load()?;
    let runtime = tokio::runtime::Runtime::new()?;
    let is_demo = std::env::var_os("CINEPLANET_DEMO").is_some();
    let (client, catalog) = if is_demo {
        (None, demo::catalog())
    } else {
        let (client, catalog) = runtime.block_on(live::load_catalog())?;
        (Some(client), catalog)
    };
    let mut app = if is_demo {
        App::new(catalog, preferences)
    } else {
        App::live(catalog, preferences)
    };
    ratatui::run(|terminal| run(terminal, &mut app, client.as_ref(), &runtime))
}

fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    client: Option<&live::CineplanetClient>,
    runtime: &tokio::runtime::Runtime,
) -> Result<()> {
    while !app.should_quit() {
        terminal.draw(|frame| ui::render(frame, app))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let Some(action) = action_for(key) else {
            continue;
        };
        match app.apply(action)? {
            Effect::None => {}
            Effect::SavePreferences => settings::save(app.preferences())?,
            Effect::FetchSeatMaps(_) => {
                terminal.draw(|frame| ui::render(frame, app))?;
                let showtimes = app.selected_showtimes();
                let hydrated = match client {
                    Some(client) => runtime.block_on(client.hydrate_showtimes(&showtimes)),
                    None => Ok(showtimes),
                };
                match hydrated {
                    Ok(showtimes) => app.finish_loading_showtimes(showtimes),
                    Err(error) => {
                        app.loading_failed();
                        eprintln!("No se pudieron actualizar los asientos reales: {error:#}");
                    }
                }
            }
        }
    }
    Ok(())
}

fn action_for(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Char('Q'), _) => Some(Action::Quit),
        (KeyCode::Char('P'), _) => Some(Action::EditVenues),
        (KeyCode::Char(' '), _) => Some(Action::Toggle),
        (KeyCode::Char(character), _) => Some(Action::Character(character)),
        (KeyCode::Up, _) => Some(Action::Up),
        (KeyCode::Down, _) => Some(Action::Down),
        (KeyCode::Enter, _) => Some(Action::Confirm),
        (KeyCode::Esc, _) => Some(Action::Back),
        (KeyCode::Backspace, _) => Some(Action::Backspace),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercase_shortcut_letters_remain_available_for_movie_search() {
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(Action::Character('p'))
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Action::Character('q'))
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)),
            Some(Action::EditVenues)
        );
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT)),
            Some(Action::Quit)
        );
    }
}
