use anyhow::Result;
use cineplanet_cli::{
    app::{Action, App, Effect},
    demo, settings, ui,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

fn main() -> Result<()> {
    let preferences = settings::load()?;
    let mut app = App::new(demo::catalog(), preferences);
    ratatui::run(|terminal| run(terminal, &mut app))
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
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
        if app.apply(action)? == Effect::SavePreferences {
            settings::save(app.preferences())?;
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
