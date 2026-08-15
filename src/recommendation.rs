use std::collections::BTreeSet;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    cli::RecommendArgs,
    domain::{Catalog, Preferences, Quality, SeatingArrangement, Showtime},
    live::{HydrationFailure, HydrationOutcome},
    ranking, viewing,
};

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecommendError {
    MovieNotFound {
        query: String,
    },
    AmbiguousMovieTitle {
        title: String,
        matches: Vec<MovieMatchV1>,
    },
    RecommendFailed {
        stage: &'static str,
        message: String,
    },
}

#[derive(Debug, Serialize)]
pub struct RecommendationErrorResponseV1 {
    pub version: &'static str,
    pub error: RecommendError,
}

#[derive(Debug, Serialize)]
pub struct MovieMatchV1 {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct RecommendationResponseV1 {
    pub version: &'static str,
    pub observed_at: String,
    pub query: QueryV1,
    pub recommendations: Vec<RankedShowtimeV1>,
    pub diagnostics: DiagnosticsV1,
}

#[derive(Debug, Serialize)]
pub struct QueryV1 {
    pub movie_id: String,
    pub movie_title: String,
    pub city: String,
    pub party_size: usize,
    pub dates: Vec<String>,
    pub venues: Vec<String>,
    pub languages: Vec<String>,
    pub formats: Vec<String>,
    pub room_types: Vec<String>,
    pub favorite_venues: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct RankedShowtimeV1 {
    pub rank: usize,
    pub id: String,
    pub venue: VenueV1,
    pub starts_at: String,
    pub modality: ModalityV1,
    pub available_seat_count: usize,
    pub viewing: ViewingV1,
    pub selected_block: Option<SelectedBlockV1>,
    /// A compact, deterministic, display-ready representation of the hydrated
    /// map. It intentionally exposes no identifiers for seats other than the
    /// selected block above.
    pub seat_preview: SeatPreviewV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_handoff: Option<CheckoutHandoffV1>,
}
#[derive(Debug, Serialize)]
pub struct CheckoutHandoffV1 {
    pub movie_slug: String,
    pub cinema_id: String,
    pub session_id: String,
    pub seat_selection_url: String,
    pub selected_seat_labels: Vec<String>,
    /// Stable identifier that binds a revalidation to this official cinema
    /// session. Browser clients must reject a URL whose session id differs.
    pub session_fingerprint: String,
    pub browser_session_required: bool,
}
#[derive(Debug, Serialize)]
pub struct SeatPreviewV1 {
    pub screen: &'static str,
    pub symbols: SeatPreviewSymbolsV1,
    pub rows: Vec<SeatPreviewRowV1>,
}
#[derive(Debug, Serialize)]
pub struct SeatPreviewSymbolsV1 {
    pub available: &'static str,
    pub occupied: &'static str,
    pub accessible: &'static str,
    pub recommended: &'static str,
    pub aisle: &'static str,
}
#[derive(Debug, Serialize)]
pub struct SeatPreviewRowV1 {
    pub label: String,
    pub layout: String,
}
#[derive(Debug, Serialize)]
pub struct VenueV1 {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Serialize)]
pub struct ModalityV1 {
    pub projection_format: String,
    pub language: String,
    pub room_type: String,
}
#[derive(Debug, Serialize)]
pub struct ViewingV1 {
    pub score: f64,
    pub quality: Quality,
    pub zone: ViewingZoneV1,
    pub reason_codes: Vec<String>,
    pub reasons: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct ViewingZoneV1 {
    pub id: &'static str,
    pub selected_block_inside: bool,
}
#[derive(Debug, Serialize)]
pub struct SelectedBlockV1 {
    pub arrangement: SeatingArrangement,
    pub seats: Vec<SeatV1>,
}
#[derive(Debug, Serialize)]
pub struct SeatV1 {
    pub id: String,
    pub row: String,
    pub number: u16,
}
#[derive(Debug, Serialize)]
pub struct DiagnosticsV1 {
    pub candidate_count: usize,
    pub hydrated_count: usize,
    pub map_failures: Vec<MapFailureV1>,
}
#[derive(Debug, Serialize)]
pub struct MapFailureV1 {
    pub showtime_id: String,
    pub message: String,
}

pub fn select_candidates(
    catalog: &Catalog,
    args: &RecommendArgs,
) -> std::result::Result<(QueryV1, Vec<Showtime>, Preferences), RecommendError> {
    let movie = if let Some(id) = &args.movie_id {
        catalog
            .movies
            .iter()
            .find(|movie| movie.id == *id)
            .cloned()
            .ok_or_else(|| RecommendError::MovieNotFound { query: id.clone() })?
    } else {
        let title = args
            .movie_title
            .as_ref()
            .expect("clap requires a movie selector");
        let matches: Vec<_> = catalog
            .movies
            .iter()
            .filter(|movie| movie.title.eq_ignore_ascii_case(title))
            .collect();
        match matches.as_slice() {
            [] => {
                return Err(RecommendError::MovieNotFound {
                    query: title.clone(),
                });
            }
            [movie] => (*movie).clone(),
            many => {
                return Err(RecommendError::AmbiguousMovieTitle {
                    title: title.clone(),
                    matches: many
                        .iter()
                        .map(|movie| MovieMatchV1 {
                            id: movie.id.clone(),
                            title: movie.title.clone(),
                        })
                        .collect(),
                });
            }
        }
    };
    let preferences = Preferences {
        party_size: args.party_size,
        favorite_venue_ids: args.favorite_venues.iter().cloned().collect(),
        // These filters are applied case-insensitively below before hydration.
        // Keeping the ranking preferences empty avoids applying them again with
        // the TUI's case-sensitive persisted-preference semantics.
        ..Preferences::default()
    };
    let candidates = catalog.showtimes.iter().filter(|showtime| {
        showtime.movie_id == movie.id
            && catalog
                .venues
                .iter()
                .find(|venue| venue.id == showtime.venue_id)
                .is_some_and(|venue| venue.city.eq_ignore_ascii_case(&args.city))
            && matches_any(&args.dates, &showtime.starts_at.date_naive().to_string())
    });
    let candidates = candidates
        .filter(|showtime| {
            (args.venues.is_empty()
                || args
                    .venues
                    .iter()
                    .any(|venue| venue_matches(venue, &showtime.venue_id, &showtime.venue_name)))
                && matches_any(&args.languages, &showtime.modality.language)
                && matches_any(&args.formats, &showtime.modality.projection_format)
                && matches_any(&args.room_types, &showtime.modality.room_type)
        })
        .cloned()
        .collect();
    Ok((query(movie.id, movie.title, args), candidates, preferences))
}

fn matches_any(values: &[String], actual: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(actual))
}

fn venue_matches(query: &str, venue_id: &str, venue_name: &str) -> bool {
    if query.eq_ignore_ascii_case(venue_id) || query.eq_ignore_ascii_case(venue_name) {
        return true;
    }

    normalized_venue_alias(query)
        .zip(normalized_venue_alias(venue_name))
        .is_some_and(|(query, name)| query == name)
}

fn normalized_venue_alias(value: &str) -> Option<String> {
    let normalized = value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();
    let words = normalized
        .split_whitespace()
        .skip_while(|word| matches!(*word, "cp" | "cineplanet" | "plaza"))
        .collect::<Vec<_>>();
    (!words.is_empty()).then(|| words.join(" "))
}

fn query(movie_id: String, movie_title: String, args: &RecommendArgs) -> QueryV1 {
    QueryV1 {
        movie_id,
        movie_title,
        city: args.city.clone(),
        party_size: args.party_size,
        dates: sorted(&args.dates),
        venues: sorted(&args.venues),
        languages: sorted(&args.languages),
        formats: sorted(&args.formats),
        room_types: sorted(&args.room_types),
        favorite_venues: sorted(&args.favorite_venues),
        limit: args.limit,
    }
}
fn sorted(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Captures the UTC instant when this CLI query observed the catalog and seat
/// maps it just hydrated. Surface it as an honest RFC 3339 string so callers
/// know exactly how fresh the recommendation is and can warn when it goes
/// stale.
fn observed_at_now() -> String {
    DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339()
}

pub fn build_response(
    query: QueryV1,
    candidates: usize,
    preferences: &Preferences,
    limit: usize,
    outcome: HydrationOutcome,
) -> RecommendationResponseV1 {
    let observed_at = observed_at_now();
    let recommendations = ranking::recommend(&outcome.showtimes, preferences, limit)
        .into_iter()
        .enumerate()
        .map(|(index, recommendation)| {
            let reason_codes = reason_codes(&recommendation);
            let showtime = recommendation.showtime;
            let available_seat_count = showtime
                .seat_map
                .seats
                .iter()
                .filter(|seat| matches!(seat.state, crate::domain::SeatState::Available))
                .count();
            let selected_block_inside = viewing::selected_block_is_in_good_viewing_zone(
                &showtime.seat_map,
                &recommendation.block,
                &recommendation.arrangement,
            );
            let checkout_handoff = (recommendation.arrangement != SeatingArrangement::Scattered
                && recommendation.block.len() == query.party_size)
                .then(|| {
                    checkout_handoff(
                        &showtime,
                        recommendation.block.iter().map(seat_label).collect(),
                    )
                })
                .flatten();
            let preview_selection = if recommendation.arrangement == SeatingArrangement::Scattered {
                &[]
            } else {
                recommendation.block.as_slice()
            };
            let seat_preview = seat_preview(&showtime.seat_map, preview_selection);
            let selected_block = (recommendation.arrangement != SeatingArrangement::Scattered)
                .then(|| SelectedBlockV1 {
                    arrangement: recommendation.arrangement,
                    seats: recommendation
                        .block
                        .into_iter()
                        .map(|seat| SeatV1 {
                            id: seat.id,
                            row: seat.row,
                            number: seat.number,
                        })
                        .collect(),
                });
            RankedShowtimeV1 {
                rank: index + 1,
                id: showtime.id,
                venue: VenueV1 {
                    id: showtime.venue_id,
                    name: showtime.venue_name,
                },
                starts_at: showtime.starts_at.to_rfc3339(),
                modality: ModalityV1 {
                    projection_format: showtime.modality.projection_format,
                    language: showtime.modality.language,
                    room_type: showtime.modality.room_type,
                },
                available_seat_count,
                viewing: ViewingV1 {
                    score: recommendation.visual_score,
                    quality: recommendation.quality,
                    zone: ViewingZoneV1 {
                        id: viewing::GOOD_VIEWING_ZONE_ID,
                        selected_block_inside,
                    },
                    reason_codes,
                    reasons: recommendation.reasons,
                },
                selected_block,
                seat_preview,
                checkout_handoff,
            }
        })
        .collect();
    RecommendationResponseV1 {
        version: "v1",
        observed_at,
        query,
        recommendations,
        diagnostics: DiagnosticsV1 {
            candidate_count: candidates,
            hydrated_count: outcome.showtimes.len(),
            map_failures: outcome.failures.into_iter().map(map_failure).collect(),
        },
    }
}

fn checkout_handoff(
    showtime: &Showtime,
    selected_seat_labels: Vec<String>,
) -> Option<CheckoutHandoffV1> {
    let movie_slug = showtime.movie_details_url.as_deref()?.trim_matches('/');
    let session_id = showtime.session_id.as_deref()?;
    if movie_slug.is_empty() || session_id.is_empty() {
        return None;
    }
    let cinema_id = showtime.venue_id.clone();
    Some(CheckoutHandoffV1 {
        movie_slug: movie_slug.to_owned(),
        cinema_id: cinema_id.clone(),
        session_id: session_id.to_owned(),
        seat_selection_url: format!(
            "https://www.cineplanet.com.pe/compra/{movie_slug}/{cinema_id}/{session_id}/asientos"
        ),
        selected_seat_labels,
        session_fingerprint: format!("cineplanet:{cinema_id}:{session_id}"),
        // Cineplanet encrypts add-tickets in its browser flow; the resulting
        // guest hold belongs to that browser session and cannot be replayed.
        browser_session_required: true,
    })
}

fn seat_preview(map: &crate::domain::SeatMap, selected: &[crate::domain::Seat]) -> SeatPreviewV1 {
    use crate::domain::SeatState;

    let selected_ids: BTreeSet<_> = selected.iter().map(|seat| seat.id.as_str()).collect();
    let mut ordered_seats: Vec<_> = map.seats.iter().collect();
    ordered_seats.sort_by_key(|seat| (seat.y, seat.x, &seat.row, seat.number));

    let rows = (0..map.rows)
        .map(|y| {
            let row_seats: Vec<_> = ordered_seats
                .iter()
                .copied()
                .filter(|seat| seat.y == y)
                .collect();
            let label = row_seats
                .first()
                .map(|seat| seat.row.clone())
                .unwrap_or_default();
            let layout = (0..map.columns)
                .map(|x| match row_seats.iter().find(|seat| seat.x == x) {
                    None => ' ',
                    Some(seat) if selected_ids.contains(seat.id.as_str()) => '*',
                    Some(seat) => match seat.state {
                        SeatState::Available => '.',
                        SeatState::Occupied => '#',
                        SeatState::Accessible => 'A',
                    },
                })
                .collect();
            SeatPreviewRowV1 { label, layout }
        })
        .collect();

    SeatPreviewV1 {
        screen: "PANTALLA",
        symbols: SeatPreviewSymbolsV1 {
            available: ".",
            occupied: "#",
            accessible: "A",
            recommended: "*",
            aisle: " ",
        },
        rows,
    }
}

fn seat_label(seat: &crate::domain::Seat) -> String {
    seat.id.clone()
}
fn map_failure(failure: HydrationFailure) -> MapFailureV1 {
    MapFailureV1 {
        showtime_id: failure.showtime_id,
        message: failure.message,
    }
}
fn reason_codes(recommendation: &crate::domain::Recommendation) -> Vec<String> {
    let mut codes = match recommendation.arrangement {
        SeatingArrangement::Together => vec!["contiguous_central_block".into()],
        SeatingArrangement::AcrossAisle { .. } => vec!["across_aisle_block".into()],
        SeatingArrangement::Scattered => vec!["no_contiguous_block".into()],
    };
    if recommendation
        .reasons
        .iter()
        .any(|reason| reason == "Sede favorita")
    {
        codes.push("favorite_venue".into());
    }
    codes
}

pub fn demo_outcome(showtimes: Vec<Showtime>) -> HydrationOutcome {
    HydrationOutcome {
        showtimes,
        failures: Vec::new(),
    }
}

pub fn failure(stage: &'static str, error: impl std::fmt::Display) -> RecommendError {
    RecommendError::RecommendFailed {
        stage,
        message: error.to_string(),
    }
}

/// Error envelopes intentionally contain only strings and fixed fields, so
/// serialization to an in-memory string is infallible for our contract.
pub fn error_json(error: RecommendError) -> String {
    serde_json::to_string_pretty(&RecommendationErrorResponseV1 {
        version: "v1",
        error,
    })
    .expect("the recommendation error envelope is serializable")
}

pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_partial_hydration_failures_as_response_diagnostics() {
        let hydrated = crate::demo::catalog().showtimes.into_iter().next().unwrap();
        let response = build_response(
            QueryV1 {
                movie_id: "movie".into(),
                movie_title: "Movie".into(),
                city: "Lima".into(),
                party_size: 2,
                dates: Vec::new(),
                venues: Vec::new(),
                languages: Vec::new(),
                formats: Vec::new(),
                room_types: Vec::new(),
                favorite_venues: Vec::new(),
                limit: 3,
            },
            2,
            &Preferences::default(),
            3,
            HydrationOutcome {
                showtimes: vec![hydrated],
                failures: vec![HydrationFailure {
                    showtime_id: "showtime-1".into(),
                    message: "network unavailable".into(),
                }],
            },
        );

        assert_eq!(response.diagnostics.candidate_count, 2);
        assert_eq!(response.diagnostics.hydrated_count, 1);
        assert_eq!(
            response.diagnostics.map_failures[0].showtime_id,
            "showtime-1"
        );
    }

    #[test]
    fn observed_at_is_present_and_is_an_rfc3339_utc_string() {
        let hydrated = crate::demo::catalog().showtimes.into_iter().next().unwrap();
        let response = build_response(
            QueryV1 {
                movie_id: "movie".into(),
                movie_title: "Movie".into(),
                city: "Lima".into(),
                party_size: 2,
                dates: Vec::new(),
                venues: Vec::new(),
                languages: Vec::new(),
                formats: Vec::new(),
                room_types: Vec::new(),
                favorite_venues: Vec::new(),
                limit: 3,
            },
            1,
            &Preferences::default(),
            3,
            HydrationOutcome {
                showtimes: vec![hydrated],
                failures: Vec::new(),
            },
        );

        let parsed = DateTime::parse_from_rfc3339(&response.observed_at)
            .expect("observed_at must be an RFC 3339 string");
        assert_eq!(
            parsed.offset().utc_minus_local(),
            0,
            "observed_at must be reported in UTC (offset 0)"
        );
        let now = Utc::now();
        let drift = (parsed.with_timezone(&Utc) - now).num_seconds().abs();
        assert!(
            drift <= 5,
            "observed_at must be within a few seconds of now, drift={drift}s"
        );

        let json = to_json(&response).expect("response is serializable");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            value["observed_at"].is_string(),
            "observed_at must serialize as a JSON string, got {json}"
        );
        assert!(
            DateTime::parse_from_rfc3339(value["observed_at"].as_str().unwrap()).is_ok(),
            "observed_at JSON value must round-trip through RFC 3339"
        );
    }

    #[test]
    fn exposes_live_checkout_handoff_for_a_complete_non_scattered_block() {
        let mut hydrated = crate::demo::catalog().showtimes.into_iter().next().unwrap();
        hydrated.movie_details_url = Some("la-odisea".into());
        hydrated.venue_id = "0000000007".into();
        hydrated.session_id = Some("66776".into());
        let response = build_response(
            QueryV1 {
                movie_id: "movie".into(),
                movie_title: "Movie".into(),
                city: "Lima".into(),
                party_size: 2,
                dates: Vec::new(),
                venues: Vec::new(),
                languages: Vec::new(),
                formats: Vec::new(),
                room_types: Vec::new(),
                favorite_venues: Vec::new(),
                limit: 1,
            },
            1,
            &Preferences::default(),
            1,
            HydrationOutcome {
                showtimes: vec![hydrated],
                failures: Vec::new(),
            },
        );

        let recommendation = &response.recommendations[0];
        let selected_block = recommendation.selected_block.as_ref().unwrap();
        assert_ne!(selected_block.arrangement, SeatingArrangement::Scattered);
        assert_eq!(selected_block.seats.len(), response.query.party_size);

        let handoff = recommendation.checkout_handoff.as_ref().unwrap();
        assert_eq!(handoff.movie_slug, "la-odisea");
        assert_eq!(handoff.cinema_id, "0000000007");
        assert_eq!(handoff.session_id, "66776");
        assert_eq!(handoff.session_fingerprint, "cineplanet:0000000007:66776");
        assert_eq!(
            handoff.seat_selection_url,
            "https://www.cineplanet.com.pe/compra/la-odisea/0000000007/66776/asientos"
        );
        assert!(handoff.browser_session_required);
        let json: serde_json::Value = serde_json::from_str(&to_json(&response).unwrap()).unwrap();
        assert!(json["recommendations"][0].get("seat_map").is_none());
        assert_eq!(
            json["recommendations"][0]["seat_preview"]["screen"],
            "PANTALLA"
        );
    }

    #[test]
    fn seat_preview_preserves_aisles_states_and_recommended_seats() {
        use crate::domain::{Seat, SeatMap, SeatState};

        let map = SeatMap {
            rows: 2,
            columns: 4,
            seats: vec![
                Seat {
                    id: "A1".into(),
                    row: "A".into(),
                    number: 1,
                    x: 0,
                    y: 0,
                    state: SeatState::Available,
                },
                Seat {
                    id: "A3".into(),
                    row: "A".into(),
                    number: 3,
                    x: 2,
                    y: 0,
                    state: SeatState::Occupied,
                },
                Seat {
                    id: "B1".into(),
                    row: "B".into(),
                    number: 1,
                    x: 0,
                    y: 1,
                    state: SeatState::Accessible,
                },
                Seat {
                    id: "B2".into(),
                    row: "B".into(),
                    number: 2,
                    x: 1,
                    y: 1,
                    state: SeatState::Available,
                },
            ],
        };
        let selected = vec![map.seats[0].clone()];

        let preview = seat_preview(&map, &selected);

        assert_eq!(preview.screen, "PANTALLA");
        assert_eq!(preview.rows[0].label, "A");
        assert_eq!(preview.rows[0].layout, "* # ");
        assert_eq!(preview.rows[1].label, "B");
        assert_eq!(preview.rows[1].layout, "A.  ");
    }

    #[test]
    fn scattered_recommendation_preview_keeps_states_without_a_recommended_marker() {
        use crate::domain::SeatState;

        let mut hydrated = crate::demo::catalog().showtimes.into_iter().next().unwrap();
        for (index, seat) in hydrated.seat_map.seats.iter_mut().enumerate() {
            seat.state = if index == 0 {
                SeatState::Available
            } else {
                SeatState::Occupied
            };
        }
        let response = build_response(
            QueryV1 {
                movie_id: "movie".into(),
                movie_title: "Movie".into(),
                city: "Lima".into(),
                party_size: 2,
                dates: Vec::new(),
                venues: Vec::new(),
                languages: Vec::new(),
                formats: Vec::new(),
                room_types: Vec::new(),
                favorite_venues: Vec::new(),
                limit: 1,
            },
            1,
            &Preferences::default(),
            1,
            HydrationOutcome {
                showtimes: vec![hydrated],
                failures: Vec::new(),
            },
        );

        let recommendation = &response.recommendations[0];
        assert!(recommendation.selected_block.is_none());
        assert!(
            recommendation
                .seat_preview
                .rows
                .iter()
                .all(|row| !row.layout.contains('*'))
        );
        assert!(
            recommendation
                .seat_preview
                .rows
                .iter()
                .any(|row| row.layout.contains('.'))
        );
        assert!(
            recommendation
                .seat_preview
                .rows
                .iter()
                .any(|row| row.layout.contains('#'))
        );
    }

    #[test]
    fn venue_aliases_match_only_after_normalizing_leading_prefixes() {
        assert!(venue_matches(
            "Plaza San Miguel",
            "san-miguel",
            "CP San Miguel"
        ));
        assert!(venue_matches("Salaverry", "salaverry", "CP Salaverry"));
        assert!(venue_matches("SAN--MIGUEL", "san-miguel", "CP San Miguel"));
        assert!(!venue_matches("Miguel", "san-miguel", "CP San Miguel"));
    }
}
