use anyhow::Result;
use cineplanet_cli::{
    app::{Action, App, Effect, Screen},
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
        let Some(action) = action_for_screen(key, app.screen(), app.active_query_is_nonempty())
        else {
            continue;
        };
        match app.apply(action)? {
            Effect::None => {}
            Effect::SavePreferences => settings::save(app.preferences())?,
            Effect::FetchSeatMaps(_) => {
                terminal.draw(|frame| ui::render(frame, app))?;
                let showtimes = app.showtimes_to_hydrate();
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

fn action_for_screen(key: KeyEvent, screen: Screen, query_nonempty: bool) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Char('Q'), _) => Some(Action::Quit),
        (KeyCode::Char(' '), _)
            if query_nonempty
                || matches!(screen, Screen::CitySetup | Screen::Movies | Screen::Results) =>
        {
            Some(Action::Character(' '))
        }
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
            action_for_screen(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                Screen::Movies,
                false,
            ),
            Some(Action::Character('p'))
        );
        assert_eq!(
            action_for_screen(
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
                Screen::Movies,
                false,
            ),
            Some(Action::Character('q'))
        );
        assert_eq!(
            action_for_screen(
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
                Screen::Movies,
                false,
            ),
            Some(Action::Character('f'))
        );
        assert_eq!(
            action_for_screen(
                KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT),
                Screen::Movies,
                false,
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            action_for_screen(
                KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
                Screen::Movies,
                false,
            ),
            Some(Action::Character('F'))
        );
    }

    #[test]
    fn space_mapping_covers_each_query_capable_screen() {
        let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);

        let cases = [
            (Screen::CitySetup, false, Action::Character(' ')),
            (Screen::CitySetup, true, Action::Character(' ')),
            (Screen::VenueSetup, false, Action::Toggle),
            (Screen::VenueSetup, true, Action::Character(' ')),
            (Screen::Movies, false, Action::Character(' ')),
            (Screen::Movies, true, Action::Character(' ')),
            (Screen::DateFilter, false, Action::Toggle),
            (Screen::DateFilter, true, Action::Character(' ')),
            (Screen::VenueFilter, false, Action::Toggle),
            (Screen::VenueFilter, true, Action::Character(' ')),
            (Screen::Results, false, Action::Character(' ')),
            (Screen::Results, true, Action::Character(' ')),
        ];

        for (screen, query_nonempty, expected) in cases {
            assert_eq!(
                action_for_screen(space, screen, query_nonempty),
                Some(expected),
                "screen={screen:?}, query_nonempty={query_nonempty}"
            );
        }
    }
}
