use anyhow::Result;
use cineplanet_cli::{
    app::{Action, App, Effect, Screen},
    checkout,
    cli::{CheckoutArgs, Cli, Command},
    demo, live, recommendation, settings, ui,
};
use clap::{Parser, error::ErrorKind};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            if let Err(print_error) = error.print() {
                eprintln!("{print_error}");
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }
        Err(error)
            if std::env::args_os()
                .nth(1)
                .is_some_and(|arg| arg == "recommend" || arg == "checkout") =>
        {
            emit_recommend_error(recommendation::failure("arguments", error));
            return ExitCode::FAILURE;
        }
        Err(error) => error.exit(),
    };
    match cli.command {
        Some(Command::Recommend(args)) => match run_recommend(*args) {
            Ok(response) => match recommendation::to_json(&response) {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    emit_recommend_error(recommendation::failure("serialization", error));
                    ExitCode::FAILURE
                }
            },
            Err(error) => {
                emit_recommend_error(error);
                ExitCode::FAILURE
            }
        },
        Some(Command::Checkout(args)) => match run_checkout(*args) {
            Ok(response) => match recommendation::to_json(&response) {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    emit_recommend_error(recommendation::failure("serialization", error));
                    ExitCode::FAILURE
                }
            },
            Err(error) => {
                emit_recommend_error(error);
                ExitCode::FAILURE
            }
        },
        Some(Command::Tui) | None => match run_tui() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error:#}");
                ExitCode::FAILURE
            }
        },
    }
}

fn emit_recommend_error(error: recommendation::RecommendError) {
    eprintln!("{}", recommendation::error_json(error));
}

fn run_tui() -> Result<()> {
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

fn run_recommend(
    args: cineplanet_cli::cli::RecommendArgs,
) -> std::result::Result<recommendation::RecommendationResponseV1, recommendation::RecommendError> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| recommendation::failure("runtime", error))?;
    let is_demo = std::env::var_os("CINEPLANET_DEMO").is_some();
    let (client, catalog) = if is_demo {
        (None, demo::catalog())
    } else {
        let (client, catalog) = runtime
            .block_on(live::load_catalog())
            .map_err(|error| recommendation::failure("catalog", error))?;
        (Some(client), catalog)
    };
    let (query, candidates, preferences) = recommendation::select_candidates(&catalog, &args)?;
    let candidate_count = candidates.len();
    let outcome = match client {
        Some(client) => runtime.block_on(client.hydrate_showtimes(&candidates)),
        None => recommendation::demo_outcome(candidates),
    };
    if candidate_count > 0 && outcome.showtimes.is_empty() && !outcome.failures.is_empty() {
        let failures = outcome
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.showtime_id, failure.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(recommendation::failure("hydration", failures));
    }
    Ok(recommendation::build_response(
        query,
        candidate_count,
        &preferences,
        args.limit,
        outcome,
    ))
}

fn run_checkout(
    mut args: CheckoutArgs,
) -> std::result::Result<checkout::CheckoutResponseV1, recommendation::RecommendError> {
    if !args.yes {
        return Err(recommendation::failure(
            "confirmation",
            "la retención temporal requiere --yes después de la confirmación del usuario",
        ));
    }
    args.recommend.limit = 1_000;
    for attempt in 0..2 {
        let response = run_recommend(args.recommend.clone())?;
        let selected = response
            .recommendations
            .iter()
            .find(|recommendation| recommendation.id == args.recommendation_id)
            .ok_or_else(|| {
                recommendation::failure(
                    "revalidation",
                    "la función elegida ya no aparece entre las recomendaciones disponibles",
                )
            })?;
        let handoff = selected.checkout_handoff.as_ref().ok_or_else(|| {
            recommendation::failure(
                "revalidation",
                "la función ya no ofrece un bloque verificable para checkout",
            )
        })?;
        match checkout::open_guest_checkout(
            selected.id.clone(),
            selected.venue.name.clone(),
            selected.starts_at.clone(),
            handoff,
        ) {
            Ok(checkout) => return Ok(checkout),
            Err(error)
                if attempt == 0 && error.to_string().contains("guest_session_initialized") =>
            {
                continue;
            }
            Err(error) => return Err(recommendation::failure("checkout", error)),
        }
    }
    Err(recommendation::failure(
        "checkout",
        "Cineplanet no pudo conservar la sesión invitada después de revalidar",
    ))
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
                let outcome = match client {
                    Some(client) => runtime.block_on(client.hydrate_showtimes(&showtimes)),
                    None => recommendation::demo_outcome(showtimes),
                };
                for failure in &outcome.failures {
                    eprintln!(
                        "No se pudo actualizar {}: {}",
                        failure.showtime_id, failure.message
                    );
                }
                app.finish_loading_showtimes(outcome.showtimes);
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
