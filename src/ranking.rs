use std::collections::BTreeMap;

use crate::domain::{Preferences, Quality, Recommendation, Seat, SeatState, Showtime};

pub fn recommend(
    showtimes: &[Showtime],
    preferences: &Preferences,
    limit: usize,
) -> Vec<Recommendation> {
    if preferences.party_size == 0 || limit == 0 {
        return Vec::new();
    }

    let mut recommendations: Vec<_> = showtimes
        .iter()
        .filter(|showtime| {
            (preferences.accepted_languages.is_empty()
                || preferences
                    .accepted_languages
                    .contains(&showtime.modality.language))
                && (preferences.accepted_formats.is_empty()
                    || preferences
                        .accepted_formats
                        .contains(&showtime.modality.projection_format))
                && (preferences.accepted_room_types.is_empty()
                    || preferences
                        .accepted_room_types
                        .contains(&showtime.modality.room_type))
        })
        .filter_map(|showtime| best_recommendation(showtime, preferences))
        .collect();
    if recommendations
        .iter()
        .any(|recommendation| recommendation.quality != Quality::Unfavorable)
    {
        recommendations.retain(|recommendation| recommendation.quality != Quality::Unfavorable);
    }
    recommendations.sort_by(|left, right| right.score.total_cmp(&left.score));
    recommendations.truncate(limit);
    recommendations
}

fn best_recommendation(showtime: &Showtime, preferences: &Preferences) -> Option<Recommendation> {
    let mut rows: BTreeMap<&str, Vec<&Seat>> = BTreeMap::new();
    for seat in &showtime.seat_map.seats {
        if seat.state == SeatState::Available {
            rows.entry(&seat.row).or_default().push(seat);
        }
    }

    let mut best: Option<(f64, Vec<Seat>)> = None;
    for seats in rows.values_mut() {
        seats.sort_by_key(|seat| seat.number);
        for window in seats.windows(preferences.party_size) {
            let contiguous = window
                .windows(2)
                .all(|pair| pair[1].number == pair[0].number + 1 && pair[1].x == pair[0].x + 1);
            if !contiguous {
                continue;
            }

            let center_x =
                window.iter().map(|seat| f64::from(seat.x)).sum::<f64>() / window.len() as f64;
            let center_y =
                window.iter().map(|seat| f64::from(seat.y)).sum::<f64>() / window.len() as f64;
            let x_denominator = f64::from(showtime.seat_map.columns.saturating_sub(1).max(1));
            let y_denominator = f64::from(showtime.seat_map.rows.saturating_sub(1).max(1));
            let normalized_x = center_x / x_denominator;
            let normalized_y = center_y / y_denominator;

            if normalized_y < 0.20 {
                continue;
            }

            let score =
                100.0 - (normalized_x - 0.5).abs() * 70.0 - (normalized_y - 0.66).abs() * 45.0;
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best = Some((score, window.iter().map(|seat| (*seat).clone()).collect()));
            }
        }
    }

    let (seat_score, block) = best?;
    let quality = if seat_score >= 90.0 {
        Quality::Excellent
    } else if seat_score >= 72.0 {
        Quality::Good
    } else {
        Quality::Unfavorable
    };

    let is_favorite = preferences.favorite_venue_ids.contains(&showtime.venue_id);
    let mut reasons = vec!["Bloque contiguo cerca del centro de la sala".into()];
    let score = if is_favorite {
        reasons.push("Sede favorita".into());
        seat_score + 8.0
    } else {
        seat_score
    };

    Some(Recommendation {
        showtime: showtime.clone(),
        block,
        quality,
        reasons,
        score,
    })
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::domain::{Modality, Preferences, Seat, SeatMap, SeatState, Showtime};

    use super::recommend;

    #[test]
    fn recommends_the_contiguous_block_closest_to_the_ideal_viewing_point() {
        let mut seats = Vec::new();
        for y in 0..10 {
            for x in 0..10 {
                let number = x + 1;
                seats.push(Seat {
                    id: format!("{}{}", (b'A' + y as u8) as char, number),
                    row: ((b'A' + y as u8) as char).to_string(),
                    number,
                    x,
                    y,
                    state: SeatState::Occupied,
                });
            }
        }

        for seat in &mut seats {
            if matches!((seat.y, seat.x), (6, 4 | 5) | (9, 0 | 1)) {
                seat.state = SeatState::Available;
            }
        }

        let showtime = Showtime {
            id: "show-1".into(),
            movie_id: "movie-1".into(),
            movie_title: "Spider-Man".into(),
            venue_id: "la-molina".into(),
            venue_name: "CP La Molina".into(),
            starts_at: DateTime::parse_from_rfc3339("2026-08-10T20:30:00-05:00").unwrap(),
            modality: Modality {
                projection_format: "2D".into(),
                language: "Subtitulada".into(),
                room_type: "Regular".into(),
            },
            seat_map: SeatMap {
                rows: 10,
                columns: 10,
                seats,
            },
        };

        let recommendations = recommend(&[showtime], &Preferences::default(), 3);

        let ids: Vec<_> = recommendations[0]
            .block
            .iter()
            .map(|seat| seat.id.as_str())
            .collect();
        assert_eq!(ids, ["G5", "G6"]);
    }

    #[test]
    fn excludes_showtimes_in_languages_the_user_did_not_accept() {
        let mut preferences = Preferences::default();
        preferences.accepted_languages.insert("Subtitulada".into());

        let subtitled = showtime_with_block("sub", "Subtitulada", 6, 4);
        let dubbed = showtime_with_block("dub", "Doblada", 6, 4);

        let recommendations = recommend(&[dubbed, subtitled], &preferences, 3);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].showtime.id, "sub");
    }

    #[test]
    fn prefers_a_favorite_venue_when_seat_quality_is_close() {
        let mut preferences = Preferences::default();
        preferences.favorite_venue_ids.insert("favorite".into());

        let non_favorite = showtime_with_block_at_venue("ideal", "other", "Subtitulada", 6, 4);
        let favorite =
            showtime_with_block_at_venue("favorite-show", "favorite", "Subtitulada", 7, 4);

        let recommendations = recommend(&[non_favorite, favorite], &preferences, 3);

        assert_eq!(recommendations[0].showtime.id, "favorite-show");
        assert!(
            recommendations[0]
                .reasons
                .iter()
                .any(|reason| reason == "Sede favorita")
        );
    }

    #[test]
    fn excludes_projection_formats_and_room_types_the_user_did_not_accept() {
        let mut preferences = Preferences::default();
        preferences.accepted_formats.insert("2D".into());
        preferences.accepted_room_types.insert("Regular".into());

        let regular_2d = showtime_with_block("regular-2d", "Subtitulada", 6, 4);
        let mut three_d = showtime_with_block("three-d", "Subtitulada", 6, 4);
        three_d.modality.projection_format = "3D".into();
        let mut prime = showtime_with_block("prime", "Subtitulada", 6, 4);
        prime.modality.room_type = "Prime".into();

        let recommendations = recommend(&[three_d, prime, regular_2d], &preferences, 3);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].showtime.id, "regular-2d");
    }

    #[test]
    fn hides_unfavorable_alternatives_when_an_apt_showtime_exists() {
        let apt = showtime_with_block("apt", "Subtitulada", 6, 4);
        let unfavorable = showtime_with_block("unfavorable", "Subtitulada", 9, 0);

        let recommendations = recommend(&[unfavorable, apt], &Preferences::default(), 3);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].showtime.id, "apt");
    }

    #[test]
    fn returns_clearly_labeled_alternatives_when_no_apt_showtime_exists() {
        let unfavorable = showtime_with_block("unfavorable", "Subtitulada", 9, 0);

        let recommendations = recommend(&[unfavorable], &Preferences::default(), 3);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(
            recommendations[0].quality,
            crate::domain::Quality::Unfavorable
        );
    }

    #[test]
    fn better_seats_can_outrank_a_favorite_venue() {
        let mut preferences = Preferences::default();
        preferences.favorite_venue_ids.insert("favorite".into());

        let favorite =
            showtime_with_block_at_venue("favorite-show", "favorite", "Subtitulada", 9, 0);
        let better = showtime_with_block_at_venue("better", "other", "Subtitulada", 6, 4);

        let recommendations = recommend(&[favorite, better], &preferences, 3);

        assert_eq!(recommendations[0].showtime.id, "better");
    }

    fn showtime_with_block(id: &str, language: &str, y: u16, start_x: u16) -> Showtime {
        showtime_with_block_at_venue(id, "la-molina", language, y, start_x)
    }

    fn showtime_with_block_at_venue(
        id: &str,
        venue_id: &str,
        language: &str,
        y: u16,
        start_x: u16,
    ) -> Showtime {
        let mut seats = Vec::new();
        for row in 0..10 {
            for x in 0..10 {
                let number = x + 1;
                seats.push(Seat {
                    id: format!("{}{}", (b'A' + row as u8) as char, number),
                    row: ((b'A' + row as u8) as char).to_string(),
                    number,
                    x,
                    y: row,
                    state: if row == y
                        && matches!(x, value if value == start_x || value == start_x + 1)
                    {
                        SeatState::Available
                    } else {
                        SeatState::Occupied
                    },
                });
            }
        }

        Showtime {
            id: id.into(),
            movie_id: "movie-1".into(),
            movie_title: "Spider-Man".into(),
            venue_id: venue_id.into(),
            venue_name: "CP La Molina".into(),
            starts_at: DateTime::parse_from_rfc3339("2026-08-10T20:30:00-05:00").unwrap(),
            modality: Modality {
                projection_format: "2D".into(),
                language: language.into(),
                room_type: "Regular".into(),
            },
            seat_map: SeatMap {
                rows: 10,
                columns: 10,
                seats,
            },
        }
    }
}
