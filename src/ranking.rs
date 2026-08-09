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
        .filter_map(|showtime| best_recommendation(showtime, preferences))
        .collect();
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

    let (score, block) = best?;
    let quality = if score >= 90.0 {
        Quality::Excellent
    } else if score >= 72.0 {
        Quality::Good
    } else {
        Quality::Unfavorable
    };

    Some(Recommendation {
        showtime: showtime.clone(),
        block,
        quality,
        reasons: vec!["Bloque contiguo cerca del centro de la sala".into()],
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
}
