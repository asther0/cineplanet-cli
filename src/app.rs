use anyhow::Result;

use crate::{
    domain::{Catalog, Movie, Preferences, Recommendation},
    ranking,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    VenueSetup,
    Movies,
    Loading,
    Results,
    SeatMap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Toggle,
    Confirm,
    Character(char),
    Backspace,
    Back,
    EditVenues,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    SavePreferences,
    FetchSeatMaps(String),
}

pub struct App {
    catalog: Catalog,
    preferences: Preferences,
    screen: Screen,
    venue_index: usize,
    movie_index: usize,
    query: String,
    selected_movie_id: Option<String>,
    recommendations: Vec<Recommendation>,
    result_index: usize,
    should_quit: bool,
    is_demo: bool,
}

impl App {
    pub fn new(catalog: Catalog, preferences: Preferences) -> Self {
        Self::with_mode(catalog, preferences, true)
    }

    pub fn live(catalog: Catalog, preferences: Preferences) -> Self {
        Self::with_mode(catalog, preferences, false)
    }

    fn with_mode(catalog: Catalog, preferences: Preferences, is_demo: bool) -> Self {
        Self {
            catalog,
            preferences,
            screen: Screen::Welcome,
            venue_index: 0,
            movie_index: 0,
            query: String::new(),
            selected_movie_id: None,
            recommendations: Vec::new(),
            result_index: 0,
            should_quit: false,
            is_demo,
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    pub fn current_movie(&self) -> Option<&Movie> {
        let selected = self.selected_movie_id.as_ref()?;
        self.catalog
            .movies
            .iter()
            .find(|movie| &movie.id == selected)
    }

    pub fn recommendations(&self) -> &[Recommendation] {
        &self.recommendations
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn venue_index(&self) -> usize {
        self.venue_index
    }

    pub fn movie_index(&self) -> usize {
        self.movie_index
    }

    pub fn result_index(&self) -> usize {
        self.result_index
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn is_demo(&self) -> bool {
        self.is_demo
    }

    pub fn visible_movies(&self) -> Vec<&Movie> {
        self.filtered_movie_indices()
            .into_iter()
            .map(|index| &self.catalog.movies[index])
            .collect()
    }

    pub fn current_recommendation(&self) -> Option<&Recommendation> {
        self.recommendations.get(self.result_index)
    }

    pub fn selected_showtimes(&self) -> Vec<crate::domain::Showtime> {
        self.selected_movie_id
            .as_ref()
            .map_or_else(Vec::new, |movie_id| {
                self.catalog
                    .showtimes
                    .iter()
                    .filter(|showtime| &showtime.movie_id == movie_id)
                    .cloned()
                    .collect()
            })
    }

    pub fn finish_loading_showtimes(&mut self, showtimes: Vec<crate::domain::Showtime>) {
        let Some(movie_id) = self.selected_movie_id.as_ref() else {
            return;
        };
        self.catalog
            .showtimes
            .retain(|showtime| &showtime.movie_id != movie_id);
        self.catalog.showtimes.extend(showtimes);
        let selected = self.selected_showtimes();
        self.recommendations = ranking::recommend(&selected, &self.preferences, 3);
        self.result_index = 0;
        self.screen = Screen::Results;
    }

    pub fn loading_failed(&mut self) {
        self.screen = Screen::Movies;
    }

    pub fn apply(&mut self, action: Action) -> Result<Effect> {
        if action == Action::Quit {
            self.should_quit = true;
            return Ok(Effect::None);
        }

        match (self.screen, action) {
            (Screen::Welcome, Action::Confirm) => {
                self.screen = if self.preferences.onboarding_complete {
                    Screen::Movies
                } else {
                    Screen::VenueSetup
                };
            }
            (Screen::VenueSetup, Action::Up) => {
                if !self.catalog.venues.is_empty() {
                    self.venue_index = self
                        .venue_index
                        .checked_sub(1)
                        .unwrap_or(self.catalog.venues.len() - 1);
                }
            }
            (Screen::VenueSetup, Action::Down) => {
                if !self.catalog.venues.is_empty() {
                    self.venue_index = (self.venue_index + 1) % self.catalog.venues.len();
                }
            }
            (Screen::VenueSetup, Action::Toggle) => {
                if let Some(venue) = self.catalog.venues.get(self.venue_index)
                    && !self.preferences.favorite_venue_ids.remove(&venue.id)
                {
                    self.preferences.favorite_venue_ids.insert(venue.id.clone());
                }
            }
            (Screen::VenueSetup, Action::Confirm) => {
                self.preferences.onboarding_complete = true;
                self.screen = Screen::Movies;
                return Ok(Effect::SavePreferences);
            }
            (Screen::Movies, Action::Character(character)) => {
                self.query.push(character);
                self.movie_index = 0;
            }
            (Screen::Movies, Action::Backspace) => {
                self.query.pop();
                self.movie_index = 0;
            }
            (Screen::Movies, Action::Up) => {
                let count = self.filtered_movie_indices().len();
                if count > 0 {
                    self.movie_index = self.movie_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::Movies, Action::Down) => {
                let count = self.filtered_movie_indices().len();
                if count > 0 {
                    self.movie_index = (self.movie_index + 1) % count;
                }
            }
            (Screen::Movies, Action::Confirm) => {
                if let Some(movie_index) =
                    self.filtered_movie_indices().get(self.movie_index).copied()
                {
                    let movie_id = self.catalog.movies[movie_index].id.clone();
                    self.selected_movie_id = Some(movie_id);
                    self.screen = Screen::Loading;
                    return Ok(Effect::FetchSeatMaps(
                        self.selected_movie_id.clone().unwrap(),
                    ));
                }
            }
            (Screen::Movies, Action::EditVenues) => {
                self.screen = Screen::VenueSetup;
            }
            (Screen::Results, Action::Up) => {
                if !self.recommendations.is_empty() {
                    self.result_index = self
                        .result_index
                        .checked_sub(1)
                        .unwrap_or(self.recommendations.len() - 1);
                }
            }
            (Screen::Results, Action::Down) => {
                if !self.recommendations.is_empty() {
                    self.result_index = (self.result_index + 1) % self.recommendations.len();
                }
            }
            (Screen::Results, Action::Confirm) => {
                if self.current_recommendation().is_some() {
                    self.screen = Screen::SeatMap;
                }
            }
            (Screen::Results, Action::Back) => {
                self.screen = Screen::Movies;
            }
            (Screen::Loading, Action::Back) => self.screen = Screen::Movies,
            (Screen::Results, Action::EditVenues) => {
                self.screen = Screen::VenueSetup;
            }
            (Screen::SeatMap, Action::Back) => {
                self.screen = Screen::Results;
            }
            _ => {}
        }
        Ok(Effect::None)
    }

    fn filtered_movie_indices(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.catalog
            .movies
            .iter()
            .enumerate()
            .filter(|(_, movie)| query.is_empty() || movie.title.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{demo, domain::Preferences};

    use super::{Action, App, Effect, Screen};

    #[test]
    fn first_run_allows_selecting_multiple_favorite_venues() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        assert_eq!(app.screen(), Screen::Welcome);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        app.apply(Action::Toggle).unwrap();
        app.apply(Action::Down).unwrap();
        app.apply(Action::Toggle).unwrap();
        let effect = app.apply(Action::Confirm).unwrap();

        assert_eq!(effect, Effect::SavePreferences);
        assert_eq!(app.screen(), Screen::Movies);
        assert_eq!(app.preferences().favorite_venue_ids.len(), 2);
        assert!(app.preferences().onboarding_complete);
    }

    #[test]
    fn typing_filters_movies_and_confirming_runs_the_ranking() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..Preferences::default()
        };
        let mut app = App::new(demo::catalog(), preferences);

        assert_eq!(app.screen(), Screen::Welcome);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::Movies);
        app.apply(Action::Character('s')).unwrap();
        app.apply(Action::Character('p')).unwrap();
        let effect = app.apply(Action::Confirm).unwrap();

        assert_eq!(app.screen(), Screen::Loading);
        assert_eq!(effect, Effect::FetchSeatMaps("spider-man".into()));
        let showtimes = app.selected_showtimes();
        app.finish_loading_showtimes(showtimes);
        assert_eq!(app.screen(), Screen::Results);
        assert_eq!(app.current_movie().unwrap().id, "spider-man");
        assert!(!app.recommendations().is_empty());
        assert!(app.recommendations().len() <= 3);
    }

    #[test]
    fn welcome_is_the_initial_screen_on_first_run() {
        let app = App::new(demo::catalog(), Preferences::default());
        assert_eq!(app.screen(), Screen::Welcome);
        assert!(!app.preferences().onboarding_complete);
    }

    #[test]
    fn welcome_is_the_initial_screen_when_preferences_are_complete() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..Preferences::default()
        };
        let app = App::new(demo::catalog(), preferences);
        assert_eq!(app.screen(), Screen::Welcome);
    }

    #[test]
    fn welcome_confirm_transitions_to_venue_setup_on_first_run() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        app.apply(Action::Confirm).unwrap();

        assert_eq!(app.screen(), Screen::VenueSetup);
        assert!(!app.preferences().onboarding_complete);
    }

    #[test]
    fn welcome_confirm_transitions_to_movies_when_preferences_are_complete() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..Preferences::default()
        };
        let mut app = App::new(demo::catalog(), preferences);

        app.apply(Action::Confirm).unwrap();

        assert_eq!(app.screen(), Screen::Movies);
    }

    #[test]
    fn welcome_ignores_actions_other_than_confirm() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        for action in [
            Action::Up,
            Action::Down,
            Action::Toggle,
            Action::Character('a'),
            Action::Backspace,
            Action::Back,
            Action::EditVenues,
        ] {
            app.apply(action).unwrap();
        }

        assert_eq!(app.screen(), Screen::Welcome);
        assert!(!app.should_quit());
        assert!(app.preferences().favorite_venue_ids.is_empty());
        assert!(app.query().is_empty());
    }
}
