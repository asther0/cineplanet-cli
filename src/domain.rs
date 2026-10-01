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

impl Showtime {
    /// Enlace al mapa de butacas de esta función en Cineplanet.
    ///
    /// La compra sigue ocurriendo fuera de la CLI, pero este enlace evita
    /// rehacer a mano la elección de ciudad, película, fecha, sede y hora: cae
    /// directo en la función ya elegida. Cineplanet retiene las butacas unos
    /// minutos desde que se abre.
    ///
    /// `id` viene compuesto como `sede-sesión`; la URL sólo quiere la sesión.
    pub fn purchase_url(&self, movie_slug: &str) -> String {
        let session_id = self
            .id
            .rsplit_once('-')
            .map_or(self.id.as_str(), |(_, id)| id);
        format!(
            "https://www.cineplanet.com.pe/compra/{movie_slug}/{}/{session_id}/asientos",
            self.venue_id
        )
    }
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
    use super::{Modality, Recommendation, SeatMap, SeatingArrangement, Showtime};
    use chrono::DateTime;

    fn showtime_with_id(id: &str, venue_id: &str) -> Showtime {
        Showtime {
            id: id.into(),
            movie_id: "movie-1".into(),
            movie_title: "La Odisea".into(),
            movie_details_url: Some("la-odisea".into()),
            session_id: Some("66776".into()),
            venue_id: venue_id.into(),
            venue_name: "CP Salaverry".into(),
            starts_at: DateTime::parse_from_rfc3339("2026-08-16T16:30:00-05:00").unwrap(),
            modality: Modality {
                projection_format: "2D".into(),
                language: "SUBTITULADA".into(),
                room_type: "Regular".into(),
            },
            seat_map: SeatMap {
                rows: 0,
                columns: 0,
                seats: Vec::new(),
            },
        }
    }

    #[test]
    fn builds_the_purchase_url_from_the_session_half_of_the_id() {
        let showtime = showtime_with_id("0000000026-95087", "0000000026");

        assert_eq!(
            showtime.purchase_url("la-odisea"),
            "https://www.cineplanet.com.pe/compra/la-odisea/0000000026/95087/asientos"
        );
    }

    #[test]
    fn falls_back_to_the_whole_id_when_it_carries_no_venue_prefix() {
        let showtime = showtime_with_id("95087", "0000000026");

        assert_eq!(
            showtime.purchase_url("la-odisea"),
            "https://www.cineplanet.com.pe/compra/la-odisea/0000000026/95087/asientos"
        );
    }

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
