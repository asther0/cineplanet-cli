use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movie {
    pub id: String,
    pub title: String,
    pub duration_minutes: Option<u16>,
    pub genre: Option<String>,
    pub rating: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Venue {
    pub id: String,
    pub name: String,
    pub city: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeatState {
    Available,
    Occupied,
    Accessible,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seat {
    pub id: String,
    pub row: String,
    pub number: u16,
    pub x: u16,
    pub y: u16,
    pub state: SeatState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatMap {
    pub rows: u16,
    pub columns: u16,
    pub seats: Vec<Seat>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modality {
    pub projection_format: String,
    pub language: String,
    pub room_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Showtime {
    pub id: String,
    pub movie_id: String,
    pub movie_title: String,
    pub venue_id: String,
    pub venue_name: String,
    pub starts_at: DateTime<FixedOffset>,
    pub modality: Modality,
    pub seat_map: SeatMap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub onboarding_complete: bool,
    pub party_size: usize,
    pub favorite_venue_ids: BTreeSet<String>,
    pub accepted_languages: BTreeSet<String>,
    pub accepted_formats: BTreeSet<String>,
    pub accepted_room_types: BTreeSet<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            party_size: 2,
            favorite_venue_ids: BTreeSet::new(),
            accepted_languages: BTreeSet::new(),
            accepted_formats: BTreeSet::new(),
            accepted_room_types: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Excellent,
    Good,
    Unfavorable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub showtime: Showtime,
    pub block: Vec<Seat>,
    pub quality: Quality,
    pub reasons: Vec<String>,
    #[serde(skip)]
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub movies: Vec<Movie>,
    pub venues: Vec<Venue>,
    pub showtimes: Vec<Showtime>,
}
