//! Read-only adapter for the public Cineplanet web contract.
//!
//! The endpoints used here are not an official third-party API.  They are
//! deliberately kept in one module so a contract change fails loudly instead
//! of making the terminal UI guess from HTML.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use chrono::DateTime;
use futures_util::{StreamExt, TryStreamExt, stream};
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

    pub async fn hydrate_showtimes(&self, showtimes: &[Showtime]) -> Result<Vec<Showtime>> {
        stream::iter(showtimes.iter().cloned().map(|showtime| {
            let client = self.clone();
            async move { client.hydrate_showtime(showtime).await }
        }))
        .buffer_unordered(8)
        .try_collect()
        .await
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
                        venue_id: cinema.cinema_id.clone(),
                        venue_name: (*venue_name).into(),
                        starts_at,
                        modality: Modality {
                            projection_format: projection_format(&session.formats),
                            language: session
                                .languages
                                .first()
                                .cloned()
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
    let mut row_index = 0_u16;
    let mut columns = 0_u16;
    for area in layout.areas {
        for row in area.rows {
            let name = row.physical_name;
            for seat in row.seats {
                let x = u16::try_from(seat.position.column_index)
                    .context("columna de asiento inválida")?;
                columns = columns.max(x.saturating_add(1));
                seats.push(Seat {
                    id: format!("{name}{}", seat.id),
                    row: name.clone(),
                    number: x.saturating_add(1),
                    x,
                    y: row_index,
                    state: if seat.status == 0 {
                        SeatState::Available
                    } else {
                        SeatState::Occupied
                    },
                });
            }
            row_index = row_index.saturating_add(1);
        }
    }
    if seats.is_empty() {
        bail!("el mapa de Cineplanet no tiene asientos")
    }
    Ok(SeatMap {
        rows: row_index,
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
    rows: Vec<SeatRow>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeatRow {
    physical_name: String,
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
    column_index: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_realistic_seat_layout_without_exposing_remote_data() {
        let response: SeatPlanResponse = serde_json::from_str(r#"{"ResponseCode":"0","ErrorDescription":null,"SeatLayoutData":{"Areas":[{"Rows":[{"PhysicalName":"G","Seats":[{"Id":"7","Status":0,"Position":{"ColumnIndex":6}},{"Id":"8","Status":1,"Position":{"ColumnIndex":7}}]}]}]}}"#).unwrap();
        let map = seat_map_from(response.seat_layout_data.unwrap()).unwrap();
        assert_eq!(map.rows, 1);
        assert_eq!(map.columns, 8);
        assert_eq!(map.seats[0].state, SeatState::Available);
        assert_eq!(map.seats[1].state, SeatState::Occupied);
    }

    #[tokio::test]
    #[ignore = "consulta el contrato público actual de Cineplanet"]
    async fn public_catalog_contract_still_parses() {
        let (_, catalog) = load_catalog().await.unwrap();
        assert!(!catalog.movies.is_empty());
        assert!(!catalog.venues.is_empty());
        assert!(!catalog.showtimes.is_empty());
    }
}
