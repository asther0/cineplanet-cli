use chrono::{Duration, Local};

use crate::domain::{Catalog, Modality, Movie, Seat, SeatMap, SeatState, Showtime, Venue};

pub fn catalog() -> Catalog {
    let movies = vec![
        Movie {
            id: "spider-man".into(),
            title: "Spider-Man: Un nuevo día".into(),
            movie_details_url: None,
            duration_minutes: Some(145),
            genre: Some("Acción".into()),
            rating: Some("APT".into()),
        },
        Movie {
            id: "odyssey".into(),
            title: "La Odisea".into(),
            movie_details_url: None,
            duration_minutes: Some(132),
            genre: Some("Aventura".into()),
            rating: Some("+14".into()),
        },
        Movie {
            id: "toy-story".into(),
            title: "Toy Story 5".into(),
            movie_details_url: None,
            duration_minutes: Some(104),
            genre: Some("Animación".into()),
            rating: Some("APT".into()),
        },
    ];

    let venues = vec![
        venue("la-molina", "CP La Molina"),
        venue("alcazar", "CP Alcázar"),
        venue("salaverry", "CP Salaverry"),
        venue("san-miguel", "CP San Miguel"),
    ];

    let now = Local::now().fixed_offset();
    let specs = [
        (
            "spider-man",
            "la-molina",
            3,
            20,
            "2D",
            "Subtitulada",
            "Regular",
            6,
            4,
        ),
        (
            "spider-man",
            "alcazar",
            18,
            19,
            "2D",
            "Subtitulada",
            "Regular",
            7,
            4,
        ),
        (
            "spider-man",
            "salaverry",
            28,
            21,
            "2D",
            "Subtitulada",
            "Prime",
            6,
            3,
        ),
        (
            "spider-man",
            "san-miguel",
            45,
            17,
            "3D",
            "Doblada",
            "Regular",
            8,
            1,
        ),
        (
            "odyssey",
            "la-molina",
            6,
            18,
            "2D",
            "Subtitulada",
            "Regular",
            6,
            4,
        ),
        (
            "odyssey",
            "salaverry",
            24,
            21,
            "2D",
            "Subtitulada",
            "Prime",
            7,
            4,
        ),
        (
            "toy-story",
            "alcazar",
            4,
            16,
            "2D",
            "Doblada",
            "Regular",
            6,
            4,
        ),
        (
            "toy-story",
            "san-miguel",
            26,
            19,
            "2D",
            "Doblada",
            "Regular",
            7,
            3,
        ),
    ];

    let showtimes = specs
        .iter()
        .enumerate()
        .map(
            |(
                index,
                (movie_id, venue_id, hours, display_hour, format, language, room, row, start_x),
            )| {
                let movie = movies.iter().find(|movie| movie.id == *movie_id).unwrap();
                let venue = venues.iter().find(|venue| venue.id == *venue_id).unwrap();
                Showtime {
                    id: format!("demo-{index}"),
                    movie_id: (*movie_id).into(),
                    movie_title: movie.title.clone(),
                    movie_details_url: movie.movie_details_url.clone(),
                    venue_id: (*venue_id).into(),
                    venue_name: venue.name.clone(),
                    session_id: None,
                    starts_at: now + Duration::hours(*hours) + Duration::minutes(*display_hour),
                    modality: Modality {
                        projection_format: (*format).into(),
                        language: (*language).into(),
                        room_type: (*room).into(),
                    },
                    seat_map: seat_map(*row, *start_x, index as u16),
                }
            },
        )
        .collect();

    Catalog {
        movies,
        venues,
        showtimes,
    }
}

fn venue(id: &str, name: &str) -> Venue {
    Venue {
        id: id.into(),
        name: name.into(),
        city: "Lima".into(),
    }
}

fn seat_map(recommended_row: u16, recommended_start_x: u16, seed: u16) -> SeatMap {
    let mut seats = Vec::new();
    for y in 0..10 {
        for x in 0..12 {
            let number = x + 1;
            let recommended =
                y == recommended_row && (x == recommended_start_x || x == recommended_start_x + 1);
            let occupied = (x + y * 3 + seed).is_multiple_of(5);
            seats.push(Seat {
                id: format!("{}{}", (b'A' + y as u8) as char, number),
                row: ((b'A' + y as u8) as char).to_string(),
                number,
                x,
                y,
                state: if recommended || !occupied {
                    SeatState::Available
                } else {
                    SeatState::Occupied
                },
            });
        }
    }
    SeatMap {
        rows: 10,
        columns: 12,
        seats,
    }
}
