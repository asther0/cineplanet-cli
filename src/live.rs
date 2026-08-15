//! Read-only adapter for the public Cineplanet web contract.
//!
//! The endpoints used here are not an official third-party API.  They are
//! deliberately kept in one module so a contract change fails loudly instead
//! of making the terminal UI guess from HTML.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use chrono::DateTime;
use futures_util::{StreamExt, stream};
use reqwest::{
    Client,
    header::{ACCEPT, USER_AGENT},
};
use serde::Deserialize;

use crate::domain::{Catalog, Modality, Movie, Seat, SeatMap, SeatState, Showtime, Venue};

const SITE: &str = "https://www.cineplanet.com.pe/";
const API: &str = "https://www.cineplanet.com.pe/api/v1-web";
const USER_AGENT_VALUE: &str = "CineplanetCLI/0.1 (+https://github.com/asther0/cineplanet-cli)";

#[derive(Clone)]
pub struct CineplanetClient {
    client: Client,
}

#[derive(Debug, Default)]
pub struct HydrationOutcome {
    pub showtimes: Vec<Showtime>,
    pub failures: Vec<HydrationFailure>,
}

#[derive(Debug)]
pub struct HydrationFailure {
    pub showtime_id: String,
    pub message: String,
}

impl CineplanetClient {
    pub async fn connect() -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent(USER_AGENT_VALUE)
            .build()
            .context("no se pudo crear el cliente HTTP")?;
        client
            .get(SITE)
            .send()
            .await
            .context("no se pudo iniciar una sesión pública con Cineplanet")?
            .error_for_status()
            .context("Cineplanet rechazó la sesión pública")?;
        Ok(Self { client })
    }

    pub async fn load_catalog(&self) -> Result<Catalog> {
        let movies: MoviesResponse = self.get_json("/cache/moviescache").await?;
        let cinemas: CinemasResponse = self.get_json("/cache/cinemascache").await?;
        let sessions: SessionsResponse = self.get_json("/cache/sessioncache").await?;
        build_catalog(movies, cinemas, sessions)
    }

    pub async fn hydrate_showtimes(&self, showtimes: &[Showtime]) -> HydrationOutcome {
        let results = stream::iter(showtimes.iter().cloned().map(|showtime| {
            let client = self.clone();
            async move {
                let id = showtime.id.clone();
                (id, client.hydrate_showtime(showtime).await)
            }
        }))
        .buffer_unordered(8)
        .collect::<Vec<(String, Result<Showtime>)>>()
        .await;
        let mut outcome = HydrationOutcome::default();
        for (showtime_id, result) in results {
            match result {
                Ok(showtime) => outcome.showtimes.push(showtime),
                Err(error) => outcome.failures.push(HydrationFailure {
                    showtime_id,
                    message: error.to_string(),
                }),
            }
        }
        outcome
            .showtimes
            .sort_by(|left, right| left.id.cmp(&right.id));
        outcome
            .failures
            .sort_by(|left, right| left.showtime_id.cmp(&right.showtime_id));
        outcome
    }

    async fn hydrate_showtime(&self, showtime: Showtime) -> Result<Showtime> {
        let raw_session_id = showtime
            .id
            .rsplit_once('-')
            .map(|(_, id)| id)
            .context("la función real no contiene un identificador de sesión válido")?;
        let path = format!(
            "/seatplan/cinema/{}/session/{raw_session_id}",
            showtime.venue_id
        );
        let response: SeatPlanResponse = self.get_json(&path).await?;
        apply_seat_plan(showtime, response)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        self.client
            .get(format!("{API}{path}"))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()
            .await
            .with_context(|| format!("no se pudo consultar {path} en Cineplanet"))?
            .error_for_status()
            .with_context(|| format!("Cineplanet rechazó {path}"))?
            .json()
            .await
            .with_context(|| format!("Cineplanet cambió el formato de {path}"))
    }
}

fn apply_seat_plan(showtime: Showtime, response: SeatPlanResponse) -> Result<Showtime> {
    if response.response_code != "0" {
        bail!(
            "Cineplanet no entregó el mapa de {}: {}",
            showtime.venue_name,
            response
                .error_description
                .unwrap_or_else(|| "respuesta inválida".into())
        );
    }
    let seat_map = seat_map_from(
        response
            .seat_layout_data
            .context("Cineplanet no devolvió asientos")?,
    )?;
    let mut showtime = showtime;
    showtime.seat_map = seat_map;
    Ok(showtime)
}

pub async fn load_catalog() -> Result<(CineplanetClient, Catalog)> {
    let client = CineplanetClient::connect().await?;
    let catalog = client.load_catalog().await?;
    Ok((client, catalog))
}

fn build_catalog(
    movies: MoviesResponse,
    cinemas: CinemasResponse,
    sessions: SessionsResponse,
) -> Result<Catalog> {
    let venues: Vec<Venue> = cinemas
        .cinemas
        .into_iter()
        .map(|cinema| Venue {
            id: cinema.id,
            name: cinema.name,
            city: cinema.city,
        })
        .collect();
    let venue_names: HashMap<_, _> = venues
        .iter()
        .map(|venue| (venue.id.as_str(), venue.name.as_str()))
        .collect();
    let session_by_id: HashMap<_, _> = sessions
        .sessions
        .into_iter()
        .map(|session| (session.id.clone(), session))
        .collect();
    let mut showtimes = Vec::new();
    let mut domain_movies = Vec::new();

    for movie in movies
        .movies
        .into_iter()
        .filter(|movie| !movie.is_coming_soon)
    {
        let title = movie.title.clone();
        for cinema in &movie.cinemas {
            let Some(venue_name) = venue_names.get(cinema.cinema_id.as_str()) else {
                continue;
            };
            for date in &cinema.dates {
                for session_id in &date.sessions {
                    let Some(session) = session_by_id.get(session_id) else {
                        continue;
                    };
                    let starts_at = DateTime::parse_from_rfc3339(&session.showtime)
                        .with_context(|| format!("hora inválida para la función {session_id}"))?;
                    showtimes.push(Showtime {
                        id: session.id.clone(),
                        movie_id: movie.id.clone(),
                        movie_title: title.clone(),
                        movie_details_url: movie.movie_details_url.clone(),
                        venue_id: cinema.cinema_id.clone(),
                        venue_name: (*venue_name).into(),
                        session_id: checkout_session_id(
                            session.session_id.as_deref(),
                            &session.id,
                        ),
                        starts_at,
                        modality: Modality {
                            projection_format: projection_format(&session.formats),
                            language: session
                                .languages
                                .first()
                                .map(|lang| normalize_language(lang))
                                .unwrap_or_else(|| "Sin especificar".into()),
                            room_type: room_type(&session.formats, &session.screen_name),
                        },
                        seat_map: SeatMap {
                            rows: 0,
                            columns: 0,
                            seats: Vec::new(),
                        },
                    });
                }
            }
        }
        domain_movies.push(Movie {
            id: movie.id,
            title,
            movie_details_url: movie.movie_details_url,
            duration_minutes: movie.run_time,
            genre: movie.genre,
            rating: movie.rating_description,
        });
    }
    if domain_movies.is_empty() || showtimes.is_empty() {
        bail!("Cineplanet no publicó películas con funciones utilizables")
    }
    Ok(Catalog {
        movies: domain_movies,
        venues,
        showtimes,
    })
}

fn normalize_language(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.to_ascii_uppercase().as_str() {
        "SUBTITULAD" | "SUBTITULADA" => "Subtitulada".into(),
        "DOBLADA" | "DOBLADO" => "Doblada".into(),
        "" => "Sin especificar".into(),
        _ => trimmed.into(),
    }
}

fn checkout_session_id(
    official_session_id: Option<&str>,
    cached_session_id: &str,
) -> Option<String> {
    if let Some(session_id) = official_session_id.filter(|session_id| !session_id.is_empty()) {
        return Some(session_id.to_owned());
    }

    let checkout_id = cached_session_id
        .rsplit_once('-')
        .map(|(_, id)| id)
        .unwrap_or(cached_session_id);
    (!checkout_id.is_empty()).then(|| checkout_id.to_owned())
}

fn projection_format(formats: &[String]) -> String {
    if formats
        .iter()
        .any(|format| format.eq_ignore_ascii_case("3D"))
    {
        "3D".into()
    } else {
        "2D".into()
    }
}

fn room_type(formats: &[String], screen_name: &str) -> String {
    formats
        .iter()
        .find(|format| !matches!(format.as_str(), "2D" | "3D" | "REGULAR"))
        .cloned()
        .unwrap_or_else(|| {
            if screen_name.to_ascii_uppercase().contains("PRIME") {
                "Prime".into()
            } else {
                "Regular".into()
            }
        })
}

fn seat_map_from(layout: SeatLayout) -> Result<SeatMap> {
    let mut seats = Vec::new();
    let mut rows = 0_u16;
    let mut columns = 0_u16;
    let mut y_offset = 0_u16;

    // Cineplanet entrega coordenadas dentro de cada área. Las áreas múltiples
    // se apilan respetando el orden vertical oficial y dejando un pasillo entre
    // ellas; la gran mayoría de salas públicas usa una sola área.
    let mut areas = layout.areas;
    areas.sort_by_key(|area| (area.top, area.left, area.number));

    for area in areas {
        let area_rows =
            u16::try_from(area.row_count.max(0)).context("cantidad de filas del área inválida")?;
        let area_columns = u16::try_from(area.column_count.max(0))
            .context("cantidad de columnas del área inválida")?;
        let inferred_rows = area
            .rows
            .iter()
            .flat_map(|row| row.seats.iter())
            .filter_map(|seat| u16::try_from(seat.position.row_index).ok())
            .max()
            .map_or(0, |row| row.saturating_add(1));
        let inferred_columns = area
            .rows
            .iter()
            .flat_map(|row| row.seats.iter())
            .filter_map(|seat| u16::try_from(seat.position.column_index).ok())
            .max()
            .map_or(0, |column| column.saturating_add(1));
        let area_rows = area_rows.max(inferred_rows);
        let area_columns = area_columns.max(inferred_columns);

        for row in area.rows {
            let Some(name) = row.physical_name else {
                continue;
            };
            for seat in row.seats {
                let state = match seat.status {
                    0 => SeatState::Available,
                    1 => SeatState::Occupied,
                    3 => SeatState::Accessible,
                    5 | 7 => continue,
                    status => bail!("estado de asiento de Cineplanet desconocido: {status}"),
                };
                let source_x = u16::try_from(seat.position.column_index)
                    .context("columna de asiento inválida")?;
                let x = area_columns.saturating_sub(1).saturating_sub(source_x);
                let source_y =
                    u16::try_from(seat.position.row_index).context("fila de asiento inválida")?;
                let y =
                    y_offset.saturating_add(area_rows.saturating_sub(1).saturating_sub(source_y));
                seats.push(Seat {
                    id: format!("{name}{}", seat.id),
                    row: name.clone(),
                    number: seat
                        .id
                        .parse()
                        .unwrap_or_else(|_| source_x.saturating_add(1)),
                    x,
                    y,
                    state,
                });
            }
        }

        rows = rows.max(y_offset.saturating_add(area_rows));
        columns = columns.max(area_columns);
        y_offset = rows.saturating_add(1);
    }
    if seats.is_empty() {
        bail!("el mapa de Cineplanet no tiene asientos")
    }
    Ok(SeatMap {
        rows,
        columns,
        seats,
    })
}

#[derive(Deserialize)]
struct MoviesResponse {
    movies: Vec<ApiMovie>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMovie {
    id: String,
    title: String,
    #[serde(default)]
    movie_details_url: Option<String>,
    #[serde(default)]
    is_coming_soon: bool,
    #[serde(default)]
    run_time: Option<u16>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    rating_description: Option<String>,
    #[serde(default)]
    cinemas: Vec<MovieCinema>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovieCinema {
    cinema_id: String,
    dates: Vec<MovieDate>,
}
#[derive(Deserialize)]
struct MovieDate {
    sessions: Vec<String>,
}
#[derive(Deserialize)]
struct CinemasResponse {
    cinemas: Vec<ApiCinema>,
}
#[derive(Deserialize)]
struct ApiCinema {
    #[serde(rename = "ID")]
    id: String,
    name: String,
    city: String,
}
#[derive(Deserialize)]
struct SessionsResponse {
    sessions: Vec<ApiSession>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiSession {
    id: String,
    #[serde(default)]
    session_id: Option<String>,
    showtime: String,
    #[serde(default)]
    screen_name: String,
    #[serde(default)]
    formats: Vec<String>,
    #[serde(default)]
    languages: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatPlanResponse {
    response_code: String,
    error_description: Option<String>,
    seat_layout_data: Option<SeatLayout>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatLayout {
    areas: Vec<SeatArea>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatArea {
    #[serde(default)]
    number: i32,
    #[serde(default)]
    left: i32,
    #[serde(default)]
    top: i32,
    #[serde(default)]
    column_count: i32,
    #[serde(default)]
    row_count: i32,
    rows: Vec<SeatRow>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatRow {
    physical_name: Option<String>,
    seats: Vec<ApiSeat>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiSeat {
    id: String,
    status: i32,
    position: SeatPosition,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatPosition {
    #[serde(default)]
    row_index: i32,
    column_index: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_showtime(id: &str) -> Showtime {
        Showtime {
            id: id.into(),
            movie_id: "movie-1".into(),
            movie_title: "Test Movie".into(),
            movie_details_url: None,
            venue_id: "venue-1".into(),
            venue_name: "Test Venue".into(),
            session_id: None,
            starts_at: DateTime::parse_from_rfc3339("2024-01-01T12:00:00-05:00").unwrap(),
            modality: Modality {
                projection_format: "2D".into(),
                language: "ESP".into(),
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
    fn parses_a_realistic_seat_layout_without_exposing_remote_data() {
        let response: SeatPlanResponse = serde_json::from_str(include_str!(
            "../tests/fixtures/cineplanet/seat-plan-basic.json"
        ))
        .unwrap();
        let map = seat_map_from(response.seat_layout_data.unwrap()).unwrap();
        assert_eq!(map.rows, 3);
        assert_eq!(map.columns, 10);
        assert_eq!(map.seats[0].y, 2);
        assert_eq!(map.seats[0].state, SeatState::Available);
        assert_eq!(map.seats[1].state, SeatState::Occupied);
    }

    #[test]
    fn prefers_official_checkout_session_ids_with_a_cached_id_fallback() {
        assert_eq!(
            checkout_session_id(Some("85899"), "0000000001-85899"),
            Some("85899".into())
        );
        assert_eq!(
            checkout_session_id(Some(""), "0000000007-66776"),
            Some("66776".into())
        );
        assert_eq!(checkout_session_id(None, "66776"), Some("66776".into()));
        assert_eq!(checkout_session_id(None, ""), None);
    }

    #[test]
    fn ignores_non_sellable_rows_without_a_physical_name() {
        let response: SeatPlanResponse = serde_json::from_str(r#"{"ResponseCode":"0","ErrorDescription":null,"SeatLayoutData":{"Areas":[{"RowCount":2,"Rows":[{"PhysicalName":null,"Seats":[]},{"PhysicalName":"G","Seats":[{"Id":"7","Status":0,"Position":{"RowIndex":0,"ColumnIndex":6}}]}]}]}}"#).unwrap();
        let map = seat_map_from(response.seat_layout_data.unwrap()).unwrap();

        assert_eq!(map.rows, 2);
        assert_eq!(map.seats.len(), 1);
        assert_eq!(map.seats[0].row, "G");
    }

    #[test]
    fn preserves_official_gaps_orientation_and_accessible_places() {
        let response: SeatPlanResponse = serde_json::from_str(include_str!(
            "../tests/fixtures/cineplanet/seat-plan-geometry.json"
        ))
        .unwrap();

        let map = seat_map_from(response.seat_layout_data.unwrap()).unwrap();

        assert_eq!((map.rows, map.columns), (5, 9));
        assert_eq!((map.seats[0].x, map.seats[0].y), (8, 4));
        assert_eq!((map.seats[2].x, map.seats[2].y), (3, 4));
        assert!(!map.seats.iter().any(|seat| seat.x == 6 && seat.y == 4));
        assert_eq!((map.seats[3].x, map.seats[3].y), (4, 1));
        assert_eq!(map.seats[3].state, SeatState::Accessible);
    }

    #[test]
    fn normalizes_row_c_to_the_official_mirrored_geometry() {
        let row_c = [
            ("4", 0, 3),
            ("5", 0, 4),
            ("6", 0, 6),
            ("7", 0, 7),
            ("8", 0, 8),
            ("9", 0, 9),
            ("10", 0, 10),
            ("0", 3, 12),
            ("0", 3, 14),
            ("16", 0, 16),
            ("17", 0, 17),
            ("18", 0, 18),
            ("19", 0, 19),
            ("21", 0, 21),
            ("22", 0, 22),
            ("23", 0, 23),
            ("24", 0, 24),
            ("25", 0, 25),
            ("26", 0, 26),
            ("27", 0, 27),
        ]
        .into_iter()
        .map(|(id, status, column_index)| ApiSeat {
            id: id.into(),
            status,
            position: SeatPosition {
                row_index: 0,
                column_index,
            },
        })
        .collect();
        let map = seat_map_from(SeatLayout {
            areas: vec![SeatArea {
                number: 1,
                left: 0,
                top: 0,
                column_count: 28,
                row_count: 1,
                rows: vec![SeatRow {
                    physical_name: Some("C".into()),
                    seats: row_c,
                }],
            }],
        })
        .unwrap();

        let mut row_c: Vec<_> = map
            .seats
            .iter()
            .filter(|seat| seat.row == "C")
            .map(|seat| (seat.x, seat.state.clone()))
            .collect();
        row_c.sort_by_key(|(x, _)| *x);

        assert_eq!(
            row_c,
            vec![
                (0, SeatState::Available),
                (1, SeatState::Available),
                (2, SeatState::Available),
                (3, SeatState::Available),
                (4, SeatState::Available),
                (5, SeatState::Available),
                (6, SeatState::Available),
                (8, SeatState::Available),
                (9, SeatState::Available),
                (10, SeatState::Available),
                (11, SeatState::Available),
                (13, SeatState::Accessible),
                (15, SeatState::Accessible),
                (17, SeatState::Available),
                (18, SeatState::Available),
                (19, SeatState::Available),
                (20, SeatState::Available),
                (21, SeatState::Available),
                (23, SeatState::Available),
                (24, SeatState::Available),
            ]
        );
    }

    #[test]
    fn omits_status_five_placeholders_and_keeps_accessible_seats() {
        let map = seat_map_from(SeatLayout {
            areas: vec![SeatArea {
                number: 1,
                left: 0,
                top: 0,
                column_count: 3,
                row_count: 1,
                rows: vec![SeatRow {
                    physical_name: Some("C".into()),
                    seats: vec![
                        ApiSeat {
                            id: "1".into(),
                            status: 0,
                            position: SeatPosition {
                                row_index: 0,
                                column_index: 2,
                            },
                        },
                        ApiSeat {
                            id: "placeholder".into(),
                            status: 5,
                            position: SeatPosition {
                                row_index: 0,
                                column_index: 1,
                            },
                        },
                        ApiSeat {
                            id: "2".into(),
                            status: 3,
                            position: SeatPosition {
                                row_index: 0,
                                column_index: 0,
                            },
                        },
                    ],
                }],
            }],
        })
        .unwrap();

        assert_eq!(map.seats.len(), 2);
        assert!(!map.seats.iter().any(|seat| seat.id == "Cplaceholder"));
        assert!(
            map.seats
                .iter()
                .any(|seat| seat.id == "C2" && seat.state == SeatState::Accessible)
        );
    }

    #[test]
    fn omits_status_seven_sanitized_seats() {
        let map = seat_map_from(SeatLayout {
            areas: vec![SeatArea {
                number: 1,
                left: 0,
                top: 0,
                column_count: 7,
                row_count: 1,
                rows: vec![SeatRow {
                    physical_name: Some("A".into()),
                    seats: vec![ApiSeat {
                        id: "8".into(),
                        status: 7,
                        position: SeatPosition {
                            row_index: 0,
                            column_index: 6,
                        },
                    }],
                }],
            }],
        })
        .unwrap_err();

        assert!(map.to_string().contains("no tiene asientos"));
    }

    #[test]
    fn rejects_unknown_seat_statuses_as_a_contract_error() {
        let error = seat_map_from(SeatLayout {
            areas: vec![SeatArea {
                number: 1,
                left: 0,
                top: 0,
                column_count: 1,
                row_count: 1,
                rows: vec![SeatRow {
                    physical_name: Some("A".into()),
                    seats: vec![ApiSeat {
                        id: "1".into(),
                        status: 99,
                        position: SeatPosition {
                            row_index: 0,
                            column_index: 0,
                        },
                    }],
                }],
            }],
        })
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("estado de asiento de Cineplanet desconocido: 99")
        );
    }

    #[test]
    fn null_seat_plan_parsing_fails() {
        let showtime = make_test_showtime("st-1");
        let response: SeatPlanResponse = serde_json::from_str(
            r#"{"ResponseCode":"0","ErrorDescription":null,"SeatLayoutData":null}"#,
        )
        .unwrap();
        let result = apply_seat_plan(showtime, response);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("no devolvió asientos"),
            "expected missing-seat error, got: {msg}"
        );
    }

    #[test]
    fn error_response_code_fails() {
        let showtime = make_test_showtime("st-2");
        let response: SeatPlanResponse = serde_json::from_str(
            r#"{"ResponseCode":"1","ErrorDescription":"sesion no disponible","SeatLayoutData":null}"#,
        )
        .unwrap();
        let result = apply_seat_plan(showtime, response);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("no entregó el mapa"),
            "expected delivery error, got: {msg}"
        );
    }

    #[test]
    fn normalize_language_maps_known_provider_labels_to_canonical_forms() {
        assert_eq!(normalize_language("SUBTITULAD"), "Subtitulada");
        assert_eq!(normalize_language("SUBTITULADA"), "Subtitulada");
        assert_eq!(normalize_language("subtitulad"), "Subtitulada");
        assert_eq!(normalize_language("subtitulada"), "Subtitulada");
        assert_eq!(normalize_language("DOBLADA"), "Doblada");
        assert_eq!(normalize_language("DOBLADO"), "Doblada");
        assert_eq!(normalize_language("doblada"), "Doblada");
        assert_eq!(normalize_language("doblado"), "Doblada");
    }

    #[test]
    fn normalize_language_leaves_unknown_labels_trimmed_but_unchanged() {
        assert_eq!(normalize_language("ESP"), "ESP");
        assert_eq!(normalize_language("  ESP  "), "ESP");
        assert_eq!(normalize_language("Castellano"), "Castellano");
    }

    #[test]
    fn normalize_language_replaces_empty_with_sin_especificar() {
        assert_eq!(normalize_language(""), "Sin especificar");
        assert_eq!(normalize_language("   "), "Sin especificar");
    }

    #[tokio::test]
    #[ignore = "consulta el contrato público actual de Cineplanet"]
    async fn public_catalog_contract_still_parses() {
        let (_, catalog) = load_catalog().await.unwrap();
        assert!(!catalog.movies.is_empty());
        assert!(!catalog.venues.is_empty());
        assert!(!catalog.showtimes.is_empty());
    }

    #[tokio::test]
    #[ignore = "consulta un mapa de asientos público actual de Cineplanet"]
    async fn public_seat_plan_contract_still_parses() {
        let (client, catalog) = load_catalog().await.unwrap();
        let showtime = catalog.showtimes.into_iter().next().unwrap();
        let hydrated = client
            .hydrate_showtime(showtime)
            .await
            .expect("Cineplanet no entregó un mapa utilizable");

        assert!(!hydrated.seat_map.seats.is_empty());
    }
}
