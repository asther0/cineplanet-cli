use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movie {
    pub id: String,
    pub title: String,
    /// Official Cineplanet `movieDetailsUrl` slug, when the catalog was
    /// loaded from the live adapter. Demo and older persisted fixtures do not
    /// have a checkout destination.
    #[serde(default)]
    pub movie_details_url: Option<String>,
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
    /// Copied from the movie so a hydrated recommendation retains its
    /// official checkout destination without requiring a second catalog lookup.
    #[serde(default)]
    pub movie_details_url: Option<String>,
    pub venue_id: String,
    pub venue_name: String,
    /// The official session identifier used by Cineplanet's checkout URL.
    #[serde(default)]
    pub session_id: Option<String>,
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
    pub city: Option<String>,
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
            city: None,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeatingArrangement {
    Together,
    AcrossAisle {
        first: usize,
        second: usize,
    },
    #[default]
    Scattered,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub showtime: Showtime,
    pub block: Vec<Seat>,
    #[serde(default)]
    pub arrangement: SeatingArrangement,
    pub quality: Quality,
    pub reasons: Vec<String>,
    #[serde(skip)]
    pub score: f64,
    #[serde(skip)]
    pub visual_score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub movies: Vec<Movie>,
    pub venues: Vec<Venue>,
    pub showtimes: Vec<Showtime>,
}

#[cfg(test)]
mod tests {
    use super::{Recommendation, SeatingArrangement};

    #[test]
    fn serializes_each_seating_arrangement() {
        let arrangement = SeatingArrangement::AcrossAisle {
            first: 4,
            second: 1,
        };

        assert_eq!(
            serde_json::from_str::<SeatingArrangement>(
                &serde_json::to_string(&arrangement).unwrap()
            )
            .unwrap(),
            arrangement
        );
    }

    #[test]
    fn deserializes_a_legacy_recommendation_without_an_arrangement() {
        let legacy = serde_json::json!({
            "showtime": {
                "id": "showtime-1",
                "movie_id": "movie-1",
                "movie_title": "Spider-Man",
                "venue_id": "la-molina",
                "venue_name": "CP La Molina",
                "starts_at": "2026-08-10T20:30:00-05:00",
                "modality": {
                    "projection_format": "2D",
                    "language": "Subtitulada",
                    "room_type": "Regular"
                },
                "seat_map": { "rows": 10, "columns": 10, "seats": [] }
            },
            "block": [],
            "quality": "Unfavorable",
            "reasons": []
        });

        let recommendation: Recommendation = serde_json::from_value(legacy).unwrap();

        assert_eq!(recommendation.arrangement, SeatingArrangement::Scattered);
    }
}
