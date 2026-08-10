use anyhow::Result;

use crate::{
    domain::{Catalog, Movie, Preferences, Recommendation, Venue},
    ranking,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    CitySetup,
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
    city_index: usize,
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
            city_index: 0,
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

    pub fn city_index(&self) -> usize {
        self.city_index
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

    pub fn available_cities(&self) -> Vec<&str> {
        let mut cities: Vec<&str> = self
            .catalog
            .venues
            .iter()
            .map(|venue| venue.city.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        cities.sort_by(|left, right| {
            match (
                left.eq_ignore_ascii_case("lima"),
                right.eq_ignore_ascii_case("lima"),
            ) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => left.cmp(right),
            }
        });
        cities
    }

    pub fn visible_venues(&self) -> Vec<&Venue> {
        let Some(city) = self.preferences.city.as_deref() else {
            return Vec::new();
        };
        self.catalog
            .venues
            .iter()
            .filter(|venue| venue.city == city)
            .collect()
    }

    pub fn visible_movies(&self) -> Vec<&Movie> {
        self.filtered_movie_indices()
            .into_iter()
            .filter(|index| self.movie_visible_in_city(*index))
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
                let city = self.preferences.city.as_deref();
                self.catalog
                    .showtimes
                    .iter()
                    .filter(|showtime| {
                        &showtime.movie_id == movie_id
                            && match city {
                                Some(city) => self.city_for_venue(&showtime.venue_id) == Some(city),
                                None => true,
                            }
                    })
                    .cloned()
                    .collect()
            })
    }

    fn city_for_venue(&self, venue_id: &str) -> Option<&str> {
        self.catalog
            .venues
            .iter()
            .find(|venue| venue.id == venue_id)
            .map(|venue| venue.city.as_str())
    }

    fn movie_visible_in_city(&self, movie_index: usize) -> bool {
        let Some(city) = self.preferences.city.as_deref() else {
            return true;
        };
        let movie = &self.catalog.movies[movie_index];
        self.catalog.showtimes.iter().any(|showtime| {
            showtime.movie_id == movie.id && self.city_for_venue(&showtime.venue_id) == Some(city)
        })
    }

    fn saved_city_available(&self) -> bool {
        match self.preferences.city.as_deref() {
            Some(city) => self.catalog.venues.iter().any(|venue| venue.city == city),
            None => false,
        }
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
                self.screen = if !self.saved_city_available() {
                    self.city_index = self
                        .available_cities()
                        .iter()
                        .position(|city| Some(*city) == self.preferences.city.as_deref())
                        .unwrap_or(0);
                    Screen::CitySetup
                } else if !self.preferences.onboarding_complete {
                    self.venue_index = 0;
                    Screen::VenueSetup
                } else {
                    Screen::Movies
                };
            }
            (Screen::CitySetup, Action::Up) => {
                let count = self.available_cities().len();
                if count > 0 {
                    self.city_index = self.city_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::CitySetup, Action::Down) => {
                let count = self.available_cities().len();
                if count > 0 {
                    self.city_index = (self.city_index + 1) % count;
                }
            }
            (Screen::CitySetup, Action::Confirm) => {
                if let Some(city) = self.available_cities().get(self.city_index).copied() {
                    self.preferences.city = Some(city.to_string());
                    self.venue_index = 0;
                    self.screen = if self.preferences.onboarding_complete {
                        Screen::Movies
                    } else {
                        Screen::VenueSetup
                    };
                    return Ok(Effect::SavePreferences);
                }
            }
            (Screen::VenueSetup, Action::Up) => {
                let count = self.visible_venues().len();
                if count > 0 {
                    self.venue_index = self.venue_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::VenueSetup, Action::Down) => {
                let count = self.visible_venues().len();
                if count > 0 {
                    self.venue_index = (self.venue_index + 1) % count;
                }
            }
            (Screen::VenueSetup, Action::Toggle) => {
                if let Some(venue_id) = self
                    .visible_venues()
                    .get(self.venue_index)
                    .map(|venue| venue.id.clone())
                    && !self.preferences.favorite_venue_ids.remove(&venue_id)
                {
                    self.preferences.favorite_venue_ids.insert(venue_id);
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
                let count = self.visible_movies().len();
                if count > 0 {
                    self.movie_index = self.movie_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::Movies, Action::Down) => {
                let count = self.visible_movies().len();
                if count > 0 {
                    self.movie_index = (self.movie_index + 1) % count;
                }
            }
            (Screen::Movies, Action::Confirm) => {
                let movies = self.visible_movies();
                if let Some(movie) = movies.get(self.movie_index).copied() {
                    let movie_id = movie.id.clone();
                    self.selected_movie_id = Some(movie_id.clone());
                    self.screen = Screen::Loading;
                    return Ok(Effect::FetchSeatMaps(movie_id));
                }
            }
            (Screen::Movies, Action::EditVenues) => {
                self.venue_index = 0;
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
                self.venue_index = 0;
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
    use crate::{
        demo,
        domain::{Preferences, Venue},
    };

    use super::{Action, App, Effect, Screen};

    fn catalog_with_two_cities() -> crate::domain::Catalog {
        let mut catalog = demo::catalog();
        catalog.venues.push(Venue {
            id: "arequipa-center".into(),
            name: "CP Arequipa Center".into(),
            city: "Arequipa".into(),
        });
        catalog
    }

    fn preferences_with_city(city: &str) -> Preferences {
        Preferences {
            city: Some(city.into()),
            ..Preferences::default()
        }
    }

    #[test]
    fn first_run_lists_cities_with_lima_first_then_alphabetical() {
        let mut catalog = demo::catalog();
        catalog.venues.clear();
        catalog.venues.push(Venue {
            id: "tr1".into(),
            name: "CP Trujillo".into(),
            city: "Trujillo".into(),
        });
        catalog.venues.push(Venue {
            id: "l1".into(),
            name: "CP Lima".into(),
            city: "Lima".into(),
        });
        catalog.venues.push(Venue {
            id: "aq1".into(),
            name: "CP Arequipa".into(),
            city: "Arequipa".into(),
        });

        let app = App::new(catalog, Preferences::default());

        assert_eq!(app.available_cities(), vec!["Lima", "Arequipa", "Trujillo"]);
    }

    #[test]
    fn first_run_requires_selecting_a_city_before_venues() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        assert_eq!(app.screen(), Screen::Welcome);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        assert_eq!(app.preferences().city.as_deref(), Some("Lima"));
    }

    #[test]
    fn city_setup_arrows_wrap_across_distinct_cities() {
        let mut catalog = demo::catalog();
        catalog.venues.clear();
        catalog.venues.push(Venue {
            id: "l1".into(),
            name: "CP Lima".into(),
            city: "Lima".into(),
        });
        catalog.venues.push(Venue {
            id: "aq1".into(),
            name: "CP Arequipa".into(),
            city: "Arequipa".into(),
        });
        catalog.venues.push(Venue {
            id: "tr1".into(),
            name: "CP Trujillo".into(),
            city: "Trujillo".into(),
        });

        let mut app = App::new(catalog, Preferences::default());
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        assert_eq!(app.city_index(), 0);

        app.apply(Action::Up).unwrap();
        assert_eq!(app.city_index(), 2);

        app.apply(Action::Down).unwrap();
        assert_eq!(app.city_index(), 0);

        app.apply(Action::Down).unwrap();
        assert_eq!(app.city_index(), 1);

        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.preferences().city.as_deref(), Some("Arequipa"));
    }

    #[test]
    fn first_run_allows_selecting_multiple_favorite_venues() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        assert_eq!(app.screen(), Screen::Welcome);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
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
    fn venue_setup_only_lists_venues_in_the_selected_city() {
        let app = App::new(catalog_with_two_cities(), preferences_with_city("Lima"));

        let visible = app.visible_venues();

        assert!(!visible.is_empty());
        assert!(visible.iter().all(|venue| venue.city == "Lima"));
        assert!(visible.iter().all(|venue| venue.id != "arequipa-center"));
    }

    #[test]
    fn out_of_city_favorite_venues_do_not_appear_in_setup_or_rankings() {
        let mut app = App::new(
            catalog_with_two_cities(),
            Preferences {
                favorite_venue_ids: ["arequipa-center".into()].into_iter().collect(),
                ..preferences_with_city("Lima")
            },
        );

        let visible = app.visible_venues();
        assert!(visible.iter().all(|venue| venue.id != "arequipa-center"));
        assert!(
            !visible
                .iter()
                .any(|venue| app.preferences().favorite_venue_ids.contains(&venue.id))
        );
        assert!(
            app.preferences()
                .favorite_venue_ids
                .contains("arequipa-center")
        );

        let spider_man = app
            .catalog()
            .movies
            .iter()
            .find(|movie| movie.id == "spider-man")
            .unwrap()
            .id
            .clone();
        app.selected_movie_id = Some(spider_man);
        let showtimes = app.selected_showtimes();
        assert!(
            showtimes
                .iter()
                .all(|showtime| showtime.venue_id != "arequipa-center")
        );
    }

    #[test]
    fn visible_movies_filter_to_those_with_showtimes_in_the_selected_city() {
        let mut catalog = demo::catalog();
        catalog
            .showtimes
            .retain(|showtime| showtime.movie_id != "toy-story");
        catalog
            .venues
            .retain(|venue| venue.id != "alcazar" && venue.id != "san-miguel");

        let app = App::new(catalog, preferences_with_city("Lima"));
        let ids: Vec<_> = app
            .visible_movies()
            .into_iter()
            .map(|movie| movie.id.clone())
            .collect();
        assert!(ids.contains(&"spider-man".to_string()));
        assert!(ids.contains(&"odyssey".to_string()));
        assert!(!ids.contains(&"toy-story".to_string()));
    }

    #[test]
    fn typing_filters_movies_and_confirming_runs_the_ranking() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
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
            ..preferences_with_city("Lima")
        };
        let app = App::new(demo::catalog(), preferences);
        assert_eq!(app.screen(), Screen::Welcome);
    }

    #[test]
    fn welcome_confirm_transitions_to_city_setup_when_no_city_is_saved() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        app.apply(Action::Confirm).unwrap();

        assert_eq!(app.screen(), Screen::CitySetup);
        assert!(app.preferences().city.is_none());
        assert!(!app.preferences().onboarding_complete);
    }

    #[test]
    fn welcome_confirm_transitions_to_venue_setup_when_city_is_saved() {
        let mut app = App::new(demo::catalog(), preferences_with_city("Lima"));

        app.apply(Action::Confirm).unwrap();

        assert_eq!(app.screen(), Screen::VenueSetup);
    }

    #[test]
    fn welcome_confirm_transitions_to_movies_when_preferences_are_complete() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
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
