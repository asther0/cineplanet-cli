use std::collections::BTreeSet;

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
    DateFilter,
    VenueFilter,
    PartySize,
    SearchSummary,
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
    result_showtimes: Vec<crate::domain::Showtime>,
    result_index: usize,
    should_quit: bool,
    is_demo: bool,
    venue_setup_caller: Screen,
    filter_dates: Vec<String>,
    filter_venues: Vec<String>,
    selected_filter_dates: BTreeSet<String>,
    selected_filter_venues: BTreeSet<String>,
    date_filter_index: usize,
    venue_filter_index: usize,
    party_size_index: usize,
}

impl App {
    pub fn new(catalog: Catalog, preferences: Preferences) -> Self {
        Self::with_mode(catalog, preferences, true)
    }

    pub fn live(catalog: Catalog, preferences: Preferences) -> Self {
        Self::with_mode(catalog, preferences, false)
    }

    fn with_mode(catalog: Catalog, mut preferences: Preferences, is_demo: bool) -> Self {
        preferences.party_size = preferences.party_size.clamp(1, 5);
        let party_size_index = preferences.party_size - 1;
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
            result_showtimes: Vec::new(),
            result_index: 0,
            should_quit: false,
            is_demo,
            venue_setup_caller: Screen::Movies,
            filter_dates: Vec::new(),
            filter_venues: Vec::new(),
            selected_filter_dates: BTreeSet::new(),
            selected_filter_venues: BTreeSet::new(),
            date_filter_index: 0,
            venue_filter_index: 0,
            party_size_index,
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

    pub fn analyze_showtime(&self, showtime: &crate::domain::Showtime) -> Option<Recommendation> {
        ranking::analyze_showtime(showtime, &self.preferences)
    }

    pub fn result_showtimes(&self) -> &[crate::domain::Showtime] {
        &self.result_showtimes
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

    pub fn filter_dates(&self) -> &[String] {
        &self.filter_dates
    }

    pub fn filter_venues(&self) -> &[String] {
        &self.filter_venues
    }

    pub fn selected_filter_dates(&self) -> &BTreeSet<String> {
        &self.selected_filter_dates
    }

    pub fn selected_filter_venues(&self) -> &BTreeSet<String> {
        &self.selected_filter_venues
    }

    pub fn date_filter_index(&self) -> usize {
        self.date_filter_index
    }

    pub fn venue_filter_index(&self) -> usize {
        self.venue_filter_index
    }

    pub fn party_size_index(&self) -> usize {
        self.party_size_index
    }

    pub fn date_filter_on_continue(&self) -> bool {
        self.date_filter_index == self.filter_dates.len()
    }

    pub fn venue_filter_on_continue(&self) -> bool {
        self.venue_filter_index == self.filter_venues.len()
    }

    pub fn party_size_on_continue(&self) -> bool {
        self.party_size_index == 5
    }

    pub fn has_selected_dates(&self) -> bool {
        !self.selected_filter_dates.is_empty()
    }

    pub fn has_selected_venues(&self) -> bool {
        !self.selected_filter_venues.is_empty()
    }

    pub fn filters_active(&self) -> bool {
        (!self.filter_dates.is_empty()
            && self.selected_filter_dates.len() < self.filter_dates.len())
            || (!self.filter_venues.is_empty()
                && self.selected_filter_venues.len() < self.filter_venues.len())
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn is_demo(&self) -> bool {
        self.is_demo
    }

    pub fn city_setup_on_continue(&self) -> bool {
        self.city_index == self.available_cities().len()
    }

    pub fn venue_setup_on_continue(&self) -> bool {
        self.venue_index == self.visible_venues().len()
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

    pub fn current_recommendation(&self) -> Option<Recommendation> {
        let showtime = self.current_result_showtime()?;
        self.analyze_showtime(showtime)
    }

    pub fn current_result_showtime(&self) -> Option<&crate::domain::Showtime> {
        self.result_index
            .checked_sub(1)
            .and_then(|index| self.result_showtimes.get(index))
    }

    pub fn available_seat_count(&self, showtime: &crate::domain::Showtime) -> usize {
        showtime
            .seat_map
            .seats
            .iter()
            .filter(|seat| matches!(seat.state, crate::domain::SeatState::Available))
            .count()
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

    pub fn showtimes_to_hydrate(&self) -> Vec<crate::domain::Showtime> {
        let mut showtimes = self.selected_showtimes();
        showtimes.retain(|showtime| {
            self.selected_filter_dates
                .contains(&showtime.starts_at.date_naive().to_string())
        });
        showtimes.retain(|showtime| self.selected_filter_venues.contains(&showtime.venue_id));
        showtimes
    }

    fn populate_filters(&mut self) {
        let showtimes = self.selected_showtimes();
        let mut dates: BTreeSet<String> = BTreeSet::new();
        let mut venues: BTreeSet<String> = BTreeSet::new();
        for showtime in &showtimes {
            dates.insert(showtime.starts_at.date_naive().to_string());
            venues.insert(showtime.venue_id.clone());
        }
        self.filter_dates = dates.into_iter().collect();
        self.filter_venues = venues.into_iter().collect();
        self.selected_filter_dates.clear();
        self.selected_filter_venues.clear();
        self.date_filter_index = 0;
        self.venue_filter_index = 0;
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
        if self.selected_movie_id.is_none() {
            return;
        }
        for hydrated in showtimes {
            if let Some(existing) = self
                .catalog
                .showtimes
                .iter_mut()
                .find(|showtime| showtime.id == hydrated.id)
            {
                *existing = hydrated;
            } else {
                self.catalog.showtimes.push(hydrated);
            }
        }
        let selected = self.showtimes_to_hydrate();
        self.recommendations = ranking::recommend(&selected, &self.preferences, selected.len());
        self.result_showtimes = selected;
        self.result_showtimes.sort_by(|left, right| {
            left.starts_at
                .cmp(&right.starts_at)
                .then_with(|| left.venue_name.cmp(&right.venue_name))
        });
        self.result_index = 0;
        self.screen = Screen::Results;
    }

    pub fn loading_failed(&mut self) {
        self.screen = Screen::SearchSummary;
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
                    self.venue_setup_caller = Screen::CitySetup;
                    Screen::VenueSetup
                } else {
                    Screen::Movies
                };
            }
            (Screen::CitySetup, Action::Up) => {
                let count = self.available_cities().len().saturating_add(1);
                if count > 0 {
                    self.city_index = self.city_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::CitySetup, Action::Down) => {
                let count = self.available_cities().len().saturating_add(1);
                if count > 0 {
                    self.city_index = (self.city_index + 1) % count;
                }
            }
            (Screen::CitySetup, Action::Confirm) => {
                if let Some(city) = self.available_cities().get(self.city_index).copied() {
                    self.preferences.city = Some(city.to_string());
                    self.venue_index = 0;
                    return Ok(Effect::SavePreferences);
                }
                if self.city_setup_on_continue() {
                    self.screen = if self.preferences.onboarding_complete {
                        Screen::Movies
                    } else {
                        self.venue_setup_caller = Screen::CitySetup;
                        Screen::VenueSetup
                    };
                    return Ok(Effect::SavePreferences);
                }
            }
            (Screen::CitySetup, Action::Back) => {
                self.screen = Screen::Welcome;
            }
            (Screen::VenueSetup, Action::Up) => {
                let count = self.visible_venues().len().saturating_add(1);
                if count > 0 {
                    self.venue_index = self.venue_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::VenueSetup, Action::Down) => {
                let count = self.visible_venues().len().saturating_add(1);
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
                if let Some(venue_id) = self
                    .visible_venues()
                    .get(self.venue_index)
                    .map(|venue| venue.id.clone())
                {
                    if !self.preferences.favorite_venue_ids.remove(&venue_id) {
                        self.preferences.favorite_venue_ids.insert(venue_id);
                    }
                    return Ok(Effect::None);
                }
                if self.venue_setup_on_continue() {
                    self.preferences.onboarding_complete = true;
                    self.screen = Screen::Movies;
                    return Ok(Effect::SavePreferences);
                }
            }
            (Screen::VenueSetup, Action::Back) => {
                self.screen = self.venue_setup_caller;
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
                    self.selected_movie_id = Some(movie.id.clone());
                    self.populate_filters();
                    self.screen = Screen::DateFilter;
                }
            }
            (Screen::Movies, Action::EditVenues) => {
                self.venue_index = 0;
                self.venue_setup_caller = Screen::Movies;
                self.screen = Screen::VenueSetup;
            }
            (Screen::Movies, Action::Back) => {
                self.venue_index = 0;
                self.venue_setup_caller = Screen::Movies;
                self.screen = Screen::VenueSetup;
            }
            (Screen::Results, Action::Up) => {
                let count = self.result_showtimes.len() + 1;
                if count > 0 {
                    self.result_index = self.result_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::Results, Action::Down) => {
                let count = self.result_showtimes.len() + 1;
                if count > 0 {
                    self.result_index = (self.result_index + 1) % count;
                }
            }
            (Screen::Results, Action::Confirm) => {
                if self.result_index == 0 {
                    self.date_filter_index = 0;
                    self.screen = Screen::DateFilter;
                } else if self.current_result_showtime().is_some() {
                    self.screen = Screen::SeatMap;
                }
            }
            (Screen::Results, Action::Back) => {
                self.screen = Screen::Movies;
            }
            (Screen::Loading, Action::Back) => self.screen = Screen::SearchSummary,
            (Screen::Results, Action::EditVenues) => {
                self.venue_index = 0;
                self.venue_setup_caller = Screen::Results;
                self.screen = Screen::VenueSetup;
            }
            (Screen::DateFilter, Action::Up) => {
                let count = self.filter_dates.len() + 1;
                if count > 0 {
                    self.date_filter_index =
                        self.date_filter_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::DateFilter, Action::Down) => {
                let count = self.filter_dates.len() + 1;
                if count > 0 {
                    self.date_filter_index = (self.date_filter_index + 1) % count;
                }
            }
            (Screen::DateFilter, Action::Toggle) => {
                if let Some(date) = self.filter_dates.get(self.date_filter_index)
                    && !self.selected_filter_dates.remove(date)
                {
                    self.selected_filter_dates.insert(date.clone());
                }
            }
            (Screen::DateFilter, Action::Confirm) => {
                if self.date_filter_on_continue() {
                    if self.has_selected_dates() {
                        self.venue_filter_index = 0;
                        self.screen = Screen::VenueFilter;
                    }
                } else if let Some(date) = self.filter_dates.get(self.date_filter_index)
                    && !self.selected_filter_dates.remove(date)
                {
                    self.selected_filter_dates.insert(date.clone());
                }
            }
            (Screen::DateFilter, Action::Back) => {
                self.screen = Screen::Movies;
            }
            (Screen::VenueFilter, Action::Up) => {
                let count = self.filter_venues.len() + 1;
                if count > 0 {
                    self.venue_filter_index =
                        self.venue_filter_index.checked_sub(1).unwrap_or(count - 1);
                }
            }
            (Screen::VenueFilter, Action::Down) => {
                let count = self.filter_venues.len() + 1;
                if count > 0 {
                    self.venue_filter_index = (self.venue_filter_index + 1) % count;
                }
            }
            (Screen::VenueFilter, Action::Toggle) => {
                if let Some(venue) = self.filter_venues.get(self.venue_filter_index)
                    && !self.selected_filter_venues.remove(venue)
                {
                    self.selected_filter_venues.insert(venue.clone());
                }
            }
            (Screen::VenueFilter, Action::Confirm) => {
                if self.venue_filter_on_continue() {
                    if self.has_selected_venues() {
                        self.party_size_index = self.preferences.party_size.clamp(1, 5) - 1;
                        self.screen = Screen::PartySize;
                    }
                } else if let Some(venue) = self.filter_venues.get(self.venue_filter_index)
                    && !self.selected_filter_venues.remove(venue)
                {
                    self.selected_filter_venues.insert(venue.clone());
                }
            }
            (Screen::VenueFilter, Action::Back) => {
                self.screen = Screen::DateFilter;
            }
            (Screen::PartySize, Action::Up) => {
                self.party_size_index = self.party_size_index.checked_sub(1).unwrap_or(5);
            }
            (Screen::PartySize, Action::Down) => {
                self.party_size_index = (self.party_size_index + 1) % 6;
            }
            (Screen::PartySize, Action::Confirm) => {
                if self.party_size_on_continue() {
                    self.screen = Screen::SearchSummary;
                    return Ok(Effect::SavePreferences);
                }
                self.preferences.party_size = self.party_size_index + 1;
            }
            (Screen::PartySize, Action::Back) => {
                self.screen = Screen::VenueFilter;
            }
            (Screen::SearchSummary, Action::Confirm) => {
                if let Some(movie_id) = self.selected_movie_id.clone() {
                    self.screen = Screen::Loading;
                    return Ok(Effect::FetchSeatMaps(movie_id));
                }
            }
            (Screen::SearchSummary, Action::Back) => {
                self.screen = Screen::VenueFilter;
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

    fn advance_to_search_summary(app: &mut App) {
        for _ in 0..app.filter_dates().len() {
            app.apply(Action::Toggle).unwrap();
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        for _ in 0..app.filter_venues().len() {
            app.apply(Action::Toggle).unwrap();
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        while !app.party_size_on_continue() {
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::SearchSummary);
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
        assert_eq!(app.screen(), Screen::CitySetup);
        assert_eq!(app.preferences().city.as_deref(), Some("Lima"));
        app.apply(Action::Down).unwrap();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
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
        assert_eq!(app.city_index(), 3);

        app.apply(Action::Down).unwrap();
        assert_eq!(app.city_index(), 0);

        app.apply(Action::Down).unwrap();
        assert_eq!(app.city_index(), 1);

        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.preferences().city.as_deref(), Some("Arequipa"));
        assert_eq!(app.screen(), Screen::CitySetup);
    }

    #[test]
    fn first_run_allows_selecting_multiple_favorite_venues() {
        let mut app = App::new(demo::catalog(), Preferences::default());

        assert_eq!(app.screen(), Screen::Welcome);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        app.apply(Action::Down).unwrap();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        app.apply(Action::Toggle).unwrap();
        app.apply(Action::Down).unwrap();
        app.apply(Action::Toggle).unwrap();
        app.apply(Action::Down).unwrap();
        app.apply(Action::Down).unwrap();
        app.apply(Action::Down).unwrap();
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
    fn typing_filters_movies_and_confirming_starts_the_guided_search() {
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

        assert_eq!(effect, Effect::None);
        assert_eq!(app.screen(), Screen::DateFilter);
        assert_eq!(app.current_movie().unwrap().id, "spider-man");
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

    #[test]
    fn esc_navigates_back_to_preceding_screens() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);

        // Welcome -> Movies (onboarding complete)
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::Movies);

        // Movies -> VenueSetup
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        assert_eq!(app.venue_setup_caller, Screen::Movies);

        // VenueSetup -> Movies (caller)
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Movies);

        // Select a movie, then return through the guided steps.
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::DateFilter);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Movies);

        // Complete the guided path and view a recommended seat map.
        app.apply(Action::Confirm).unwrap();
        advance_to_search_summary(&mut app);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::Loading);
        let showtimes = app.showtimes_to_hydrate();
        app.finish_loading_showtimes(showtimes);
        app.apply(Action::Down).unwrap();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::SeatMap);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Results);

        // Results exposes modifying the search as the first item.
        app.apply(Action::Up).unwrap();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::DateFilter);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Movies);
    }

    #[test]
    fn city_setup_has_continue_button_and_enter_selects_without_advancing() {
        let mut app = App::new(demo::catalog(), Preferences::default());
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);

        // Confirm on city selects it but stays
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        assert_eq!(app.preferences().city.as_deref(), Some("Lima"));

        // Down moves to Continue button
        app.apply(Action::Down).unwrap();
        assert!(app.city_setup_on_continue());

        // Confirm on Continue advances
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
    }

    #[test]
    fn city_setup_esc_returns_to_welcome() {
        let mut app = App::new(demo::catalog(), Preferences::default());
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Welcome);
    }

    #[test]
    fn venue_setup_enter_toggles_venue_but_stays_enter_on_continue_advances() {
        let mut app = App::new(demo::catalog(), Preferences::default());
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Down).unwrap();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);

        // Enter on first venue toggles it
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.preferences().favorite_venue_ids.len(), 1);
        assert_eq!(app.screen(), Screen::VenueSetup);

        // Space on second venue toggles it
        app.apply(Action::Down).unwrap();
        app.apply(Action::Toggle).unwrap();
        assert_eq!(app.preferences().favorite_venue_ids.len(), 2);
        assert_eq!(app.screen(), Screen::VenueSetup);

        // Navigate to Continue and confirm
        for _ in 0..3 {
            app.apply(Action::Down).unwrap();
        }
        assert!(app.venue_setup_on_continue());
        let effect = app.apply(Action::Confirm).unwrap();
        assert_eq!(effect, Effect::SavePreferences);
        assert_eq!(app.screen(), Screen::Movies);
        assert!(app.preferences().onboarding_complete);
    }

    #[test]
    fn venue_setup_esc_returns_to_caller_during_onboarding() {
        let mut app = App::new(demo::catalog(), preferences_with_city("Lima"));
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        assert_eq!(app.venue_setup_caller, Screen::CitySetup);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::CitySetup);
    }

    #[test]
    fn venue_setup_esc_returns_to_caller_when_opened_from_movies() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::Movies);
        app.apply(Action::EditVenues).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        assert_eq!(app.venue_setup_caller, Screen::Movies);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Movies);
    }

    #[test]
    fn venue_setup_esc_returns_to_caller_when_opened_from_results() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        let showtimes = app.selected_showtimes();
        app.finish_loading_showtimes(showtimes);
        assert_eq!(app.screen(), Screen::Results);
        app.apply(Action::EditVenues).unwrap();
        assert_eq!(app.screen(), Screen::VenueSetup);
        assert_eq!(app.venue_setup_caller, Screen::Results);
        app.apply(Action::Back).unwrap();
        assert_eq!(app.screen(), Screen::Results);
    }

    #[test]
    fn movie_selection_opens_the_guided_date_step() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        let effect = app.apply(Action::Confirm).unwrap();
        assert_eq!(effect, Effect::None);
        assert_eq!(app.screen(), Screen::DateFilter);
        assert!(!app.filter_dates().is_empty());
        assert!(!app.filter_venues().is_empty());
        assert!(app.selected_filter_dates().is_empty());
        assert!(app.selected_filter_venues().is_empty());
    }

    #[test]
    fn guided_filters_scope_the_live_seat_map_request() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        let first_date = app.filter_dates()[0].clone();
        let first_venue = app.filter_venues()[0].clone();

        app.selected_filter_dates = app.filter_dates.iter().skip(1).cloned().collect();
        app.date_filter_index = app.filter_dates.len();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::VenueFilter);
        app.selected_filter_venues = app.filter_venues.iter().skip(1).cloned().collect();
        app.venue_filter_index = app.filter_venues.len();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::PartySize);
        while !app.party_size_on_continue() {
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::SearchSummary);
        let effect = app.apply(Action::Confirm).unwrap();
        assert_eq!(
            effect,
            Effect::FetchSeatMaps(app.current_movie().unwrap().id.clone())
        );
        assert_eq!(app.screen(), Screen::Loading);
        let scoped = app.showtimes_to_hydrate();
        assert!(
            scoped
                .iter()
                .all(|showtime| showtime.starts_at.date_naive().to_string() != first_date)
        );
        assert!(
            scoped
                .iter()
                .all(|showtime| showtime.venue_id != first_venue)
        );
    }

    #[test]
    fn party_size_is_chosen_before_searching_and_is_saved() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        app.selected_filter_dates = app.filter_dates.iter().cloned().collect();
        app.date_filter_index = app.filter_dates.len();
        app.apply(Action::Confirm).unwrap();
        app.selected_filter_venues = app.filter_venues.iter().cloned().collect();
        app.venue_filter_index = app.filter_venues.len();
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::PartySize);

        app.party_size_index = 3;
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.preferences().party_size, 4);
        assert_eq!(app.screen(), Screen::PartySize);

        app.party_size_index = 5;
        let effect = app.apply(Action::Confirm).unwrap();
        assert_eq!(effect, Effect::SavePreferences);
        assert_eq!(app.screen(), Screen::SearchSummary);
    }

    #[test]
    fn results_keep_every_selected_showtime_even_without_a_recommendation() {
        let preferences = Preferences {
            onboarding_complete: true,
            party_size: 5,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        app.selected_filter_dates = app.filter_dates.iter().cloned().collect();
        app.selected_filter_venues = app.filter_venues.iter().cloned().collect();
        let expected = app.showtimes_to_hydrate();
        app.finish_loading_showtimes(expected.clone());

        assert_eq!(app.result_showtimes().len(), expected.len());
        assert!(!app.result_showtimes().is_empty());
    }

    #[test]
    fn favorites_remain_only_a_ranking_bonus_without_implicit_restriction() {
        let mut preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        preferences.favorite_venue_ids.insert("la-molina".into());
        let favorite_ids = preferences.favorite_venue_ids.clone();
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();

        // Set filter to include only a non-favorite venue
        let non_favorite = app
            .filter_venues()
            .iter()
            .find(|v| !favorite_ids.contains(*v))
            .cloned()
            .unwrap();
        app.selected_filter_dates = app.filter_dates.iter().cloned().collect();
        app.selected_filter_venues.insert(non_favorite.clone());
        app.finish_loading_showtimes(app.showtimes_to_hydrate());

        assert!(!app.recommendations().is_empty());
        assert!(
            app.recommendations()
                .iter()
                .all(|rec| rec.showtime.venue_id == non_favorite)
        );
    }

    #[test]
    fn results_offer_modify_search_before_recommendations() {
        let preferences = Preferences {
            onboarding_complete: true,
            ..preferences_with_city("Lima")
        };
        let mut app = App::new(demo::catalog(), preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();
        advance_to_search_summary(&mut app);
        app.apply(Action::Confirm).unwrap();
        let showtimes = app.showtimes_to_hydrate();
        app.finish_loading_showtimes(showtimes);
        assert_eq!(app.screen(), Screen::Results);
        assert_eq!(app.result_index(), 0);
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::DateFilter);
    }
}
