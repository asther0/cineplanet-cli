use std::collections::BTreeMap;

use crate::domain::{
    Preferences, Quality, Recommendation, Seat, SeatState, SeatingArrangement, Showtime,
};

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
    let has_apt_across_aisle = recommendations.iter().any(|recommendation| {
        recommendation.quality != Quality::Unfavorable
            && matches!(
                recommendation.arrangement,
                SeatingArrangement::AcrossAisle { .. }
            )
    });
    if recommendations
        .iter()
        .any(|recommendation| recommendation.quality != Quality::Unfavorable)
    {
        recommendations.retain(|recommendation| {
            recommendation.quality != Quality::Unfavorable
                || (has_apt_across_aisle
                    && recommendation.arrangement == SeatingArrangement::Together)
        });
    }
    recommendations.sort_by(|left, right| {
        arrangement_priority(&left.arrangement)
            .cmp(&arrangement_priority(&right.arrangement))
            .then_with(|| right.score.total_cmp(&left.score))
    });
    recommendations.truncate(limit);
    recommendations
}

pub fn analyze_showtime(showtime: &Showtime, preferences: &Preferences) -> Option<Recommendation> {
    if preferences.party_size == 0 {
        return None;
    }
    best_recommendation(showtime, preferences)
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
            let contiguous = window.windows(2).all(|pair| {
                pair[1].number == pair[0].number + 1 && pair[1].x.abs_diff(pair[0].x) == 1
            });
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

    let (seat_score, block, arrangement) = if let Some((seat_score, block)) = best {
        (seat_score, block, SeatingArrangement::Together)
    } else if let Some((seat_score, block, first, second)) =
        best_across_aisle(showtime, preferences)
    {
        (
            seat_score,
            block,
            SeatingArrangement::AcrossAisle { first, second },
        )
    } else {
        return fallback_recommendation(showtime, preferences);
    };
    let quality = if seat_score >= 90.0 {
        Quality::Excellent
    } else if seat_score >= 72.0 {
        Quality::Good
    } else {
        Quality::Unfavorable
    };

    let is_favorite = preferences.favorite_venue_ids.contains(&showtime.venue_id);
    let mut reasons = vec![match arrangement {
        SeatingArrangement::Together => "Bloque contiguo cerca del centro de la sala".into(),
        SeatingArrangement::AcrossAisle { .. } => {
            "Dos bloques contiguos separados por un pasillo".into()
        }
        SeatingArrangement::Scattered => unreachable!(),
    }];
    let score = if is_favorite {
        reasons.push("Sede favorita".into());
        seat_score + 8.0
    } else {
        seat_score
    };

    Some(Recommendation {
        showtime: showtime.clone(),
        block,
        arrangement,
        quality,
        reasons,
        score,
    })
}

fn arrangement_priority(arrangement: &SeatingArrangement) -> u8 {
    match arrangement {
        SeatingArrangement::Together => 0,
        SeatingArrangement::AcrossAisle { .. } => 1,
        SeatingArrangement::Scattered => 2,
    }
}

fn best_across_aisle(
    showtime: &Showtime,
    preferences: &Preferences,
) -> Option<(f64, Vec<Seat>, usize, usize)> {
    let mut rows: BTreeMap<&str, Vec<&Seat>> = BTreeMap::new();
    for seat in &showtime.seat_map.seats {
        rows.entry(&seat.row).or_default().push(seat);
    }

    let mut best: Option<(usize, f64, Vec<Seat>, usize, usize)> = None;
    for seats in rows.values_mut() {
        seats.sort_by_key(|seat| seat.x);
        for pair in seats.windows(2).filter(|pair| pair[1].x > pair[0].x + 1) {
            let (left, right) = (pair[0], pair[1]);
            let left_run = available_run_ending_at(seats, left.x);
            let right_run = available_run_starting_at(seats, right.x);
            for left_size in 1..=left_run.len().min(preferences.party_size.saturating_sub(1)) {
                let right_size = preferences.party_size - left_size;
                if right_size == 0 || right_size > right_run.len() {
                    continue;
                }
                let mut block: Vec<Seat> = left_run[left_run.len() - left_size..]
                    .iter()
                    .chain(right_run[..right_size].iter())
                    .map(|seat| (*seat).clone())
                    .collect();
                block.sort_by_key(|seat| seat.number);
                let Some(score) = visual_score(showtime, &block) else {
                    continue;
                };
                let smaller = left_size.min(right_size);
                if best.as_ref().is_none_or(|(best_smaller, best_score, ..)| {
                    smaller < *best_smaller || (smaller == *best_smaller && score > *best_score)
                }) {
                    best = Some((smaller, score, block, left_size, right_size));
                }
            }
        }
    }
    best.map(|(_, score, block, first, second)| (score, block, first, second))
}

fn available_run_ending_at<'a>(seats: &[&'a Seat], x: u16) -> Vec<&'a Seat> {
    let Some(end) = seats.iter().position(|seat| seat.x == x) else {
        return Vec::new();
    };
    let mut start = end;
    while start > 0 && contiguous_available(seats[start - 1], seats[start]) {
        start -= 1;
    }
    if seats[end].state == SeatState::Available {
        seats[start..=end].to_vec()
    } else {
        Vec::new()
    }
}

fn available_run_starting_at<'a>(seats: &[&'a Seat], x: u16) -> Vec<&'a Seat> {
    let Some(start) = seats.iter().position(|seat| seat.x == x) else {
        return Vec::new();
    };
    let mut end = start;
    while end + 1 < seats.len() && contiguous_available(seats[end], seats[end + 1]) {
        end += 1;
    }
    if seats[start].state == SeatState::Available {
        seats[start..=end].to_vec()
    } else {
        Vec::new()
    }
}

fn contiguous_available(left: &Seat, right: &Seat) -> bool {
    left.state == SeatState::Available
        && right.state == SeatState::Available
        && right.x == left.x + 1
        && left.number.abs_diff(right.number) == 1
}

fn visual_score(showtime: &Showtime, block: &[Seat]) -> Option<f64> {
    let center_x = block.iter().map(|seat| f64::from(seat.x)).sum::<f64>() / block.len() as f64;
    let center_y = block.iter().map(|seat| f64::from(seat.y)).sum::<f64>() / block.len() as f64;
    let normalized_x = center_x / f64::from(showtime.seat_map.columns.saturating_sub(1).max(1));
    let normalized_y = center_y / f64::from(showtime.seat_map.rows.saturating_sub(1).max(1));
    (normalized_y >= 0.20)
        .then(|| 100.0 - (normalized_x - 0.5).abs() * 70.0 - (normalized_y - 0.66).abs() * 45.0)
}

fn fallback_recommendation(
    showtime: &Showtime,
    preferences: &Preferences,
) -> Option<Recommendation> {
    let available: Vec<_> = showtime
        .seat_map
        .seats
        .iter()
        .filter(|seat| seat.state == SeatState::Available)
        .cloned()
        .collect();
    let representative = available.first()?.clone();
    let available_count = available.len();
    let mut reasons = vec![format!(
        "Sin bloque contiguo para {} personas; quedan {available_count} asientos disponibles",
        preferences.party_size
    )];
    let mut score = available_count as f64;
    if preferences.favorite_venue_ids.contains(&showtime.venue_id) {
        reasons.push("Sede favorita".into());
        score += 8.0;
    }
    Some(Recommendation {
        showtime: showtime.clone(),
        block: vec![representative],
        arrangement: SeatingArrangement::Scattered,
        quality: Quality::Unfavorable,
        reasons,
        score,
    })
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::domain::{
        Modality, Preferences, Seat, SeatMap, SeatState, SeatingArrangement, Showtime,
    };

    use super::{analyze_showtime, recommend};

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
    fn recommends_a_contiguous_block_with_mirrored_x_coordinates() {
        let mut showtime = showtime_with_block("mirrored", "Subtitulada", 6, 4);
        for seat in &mut showtime.seat_map.seats {
            seat.x = 9 - seat.x;
        }

        let recommendations = recommend(&[showtime], &Preferences::default(), 3);

        let ids: Vec<_> = recommendations[0]
            .block
            .iter()
            .map(|seat| seat.id.as_str())
            .collect();
        assert_eq!(ids, ["G5", "G6"]);
    }

    #[test]
    fn recommends_four_plus_one_across_a_physical_aisle() {
        let showtime = showtime_with_row_layout(
            "aisle",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        let recommendation = analyze_showtime(&showtime, &preferences).unwrap();

        assert_eq!(
            recommendation.arrangement,
            SeatingArrangement::AcrossAisle {
                first: 4,
                second: 1
            }
        );
        assert_eq!(recommendation.block.len(), 5);
    }

    #[test]
    fn prefers_four_plus_one_over_three_plus_two_across_the_same_aisle() {
        let showtime = showtime_with_row_layout(
            "aisle",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
                (6, 6, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert_eq!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::AcrossAisle {
                first: 4,
                second: 1
            }
        );
    }

    #[test]
    fn does_not_treat_an_occupied_seat_as_an_aisle() {
        let showtime = showtime_with_row_layout(
            "occupied",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, false),
                (5, 6, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert_eq!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::Scattered
        );
    }

    #[test]
    fn requires_both_aisle_blocks_to_be_in_the_same_row() {
        let mut showtime = showtime_with_row_layout(
            "rows",
            6,
            &[(0, 1, true), (1, 2, true), (2, 3, true), (3, 4, true)],
        );
        showtime
            .seat_map
            .seats
            .extend(
                [(5, 5, true)]
                    .into_iter()
                    .map(|(x, number, available)| Seat {
                        id: format!("H{number}"),
                        row: "H".into(),
                        number,
                        x,
                        y: 7,
                        state: if available {
                            SeatState::Available
                        } else {
                            SeatState::Occupied
                        },
                    }),
            );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert_eq!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::Scattered
        );
    }

    #[test]
    fn recommends_across_an_aisle_with_mirrored_x_coordinates() {
        let showtime = showtime_with_row_layout(
            "mirrored-aisle",
            6,
            &[
                (9, 1, true),
                (8, 2, true),
                (7, 3, true),
                (6, 4, true),
                (4, 5, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert!(matches!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::AcrossAisle {
                first: 4,
                second: 1
            } | SeatingArrangement::AcrossAisle {
                first: 1,
                second: 4
            }
        ));
    }

    #[test]
    fn prefers_a_fully_contiguous_block_over_an_across_aisle_option() {
        let showtime = showtime_with_row_layout(
            "together",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
                (6, 6, true),
                (7, 7, true),
                (8, 8, true),
                (9, 9, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert_eq!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::Together
        );
    }

    #[test]
    fn ranks_a_contiguous_block_before_a_better_positioned_aisle_option() {
        let together = showtime_with_row_layout(
            "together",
            2,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, true),
            ],
        );
        let aisle = showtime_with_row_layout(
            "aisle",
            6,
            &[
                (3, 1, true),
                (4, 2, true),
                (5, 3, true),
                (6, 4, true),
                (8, 5, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        let recommendations = recommend(&[aisle, together], &preferences, 3);

        assert_eq!(recommendations[0].showtime.id, "together");
        assert_eq!(recommendations[0].arrangement, SeatingArrangement::Together);
    }

    #[test]
    fn continues_past_an_oversized_front_row_run_to_find_a_valid_aisle_row() {
        let mut showtime = showtime_with_row_layout(
            "later-aisle",
            0,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, true),
                (5, 6, true),
                (7, 7, true),
            ],
        );
        for seat in &mut showtime.seat_map.seats {
            seat.row = "A".into();
        }
        showtime.seat_map.seats.extend(row_seats(
            "G",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
            ],
        ));
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert!(matches!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::AcrossAisle { .. }
        ));
    }

    #[test]
    fn continues_to_a_later_physical_aisle_when_the_first_cannot_fit_the_group() {
        let showtime = showtime_with_row_layout(
            "second-aisle",
            6,
            &[
                (0, 1, true),
                (1, 2, true),
                (3, 3, true),
                (4, 4, true),
                (6, 5, true),
                (7, 6, true),
                (8, 7, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        assert_eq!(
            analyze_showtime(&showtime, &preferences)
                .unwrap()
                .arrangement,
            SeatingArrangement::AcrossAisle {
                first: 2,
                second: 3,
            }
        );
    }

    #[test]
    fn ranks_an_unfavorable_across_aisle_option_before_scattered_seats() {
        let across_aisle = showtime_with_row_layout(
            "across-aisle",
            9,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
            ],
        );
        let scattered =
            showtime_with_row_layout("scattered", 6, &[(0, 1, true), (2, 2, true), (4, 3, true)]);
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        let recommendations = recommend(&[scattered, across_aisle], &preferences, 3);

        assert_eq!(recommendations[0].showtime.id, "across-aisle");
        assert!(matches!(
            recommendations[0].arrangement,
            SeatingArrangement::AcrossAisle { .. }
        ));
    }

    #[test]
    fn keeps_an_unfavorable_together_option_when_an_across_aisle_option_is_apt() {
        let together = showtime_with_row_layout(
            "together",
            9,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, true),
            ],
        );
        let across_aisle = showtime_with_row_layout(
            "across-aisle",
            6,
            &[
                (3, 1, true),
                (4, 2, true),
                (5, 3, true),
                (6, 4, true),
                (8, 5, true),
            ],
        );
        let preferences = Preferences {
            party_size: 5,
            ..Preferences::default()
        };

        let recommendations = recommend(&[across_aisle, together], &preferences, 3);

        assert_eq!(
            recommendations
                .iter()
                .map(|recommendation| recommendation.showtime.id.as_str())
                .collect::<Vec<_>>(),
            ["together", "across-aisle"]
        );
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
    fn analyzes_an_unfavorable_showtime_even_when_recommend_hides_it() {
        let apt = showtime_with_block("apt", "Subtitulada", 6, 4);
        let unfavorable = showtime_with_block("unfavorable", "Subtitulada", 9, 0);
        let preferences = Preferences::default();

        assert_eq!(
            recommend(&[apt, unfavorable.clone()], &preferences, 3).len(),
            1
        );

        let analysis = analyze_showtime(&unfavorable, &preferences).unwrap();
        assert_eq!(analysis.quality, crate::domain::Quality::Unfavorable);
        assert_eq!(analysis.showtime.id, "unfavorable");
    }

    #[test]
    fn does_not_analyze_a_zero_person_party() {
        let showtime = showtime_with_block("showtime", "Subtitulada", 6, 4);
        let preferences = Preferences {
            party_size: 0,
            ..Preferences::default()
        };

        assert!(analyze_showtime(&showtime, &preferences).is_none());
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
    fn keeps_available_showtimes_when_the_group_has_no_contiguous_block() {
        let showtime = showtime_with_block("split", "Subtitulada", 6, 4);
        let preferences = Preferences {
            party_size: 3,
            ..Preferences::default()
        };

        let recommendations = recommend(&[showtime], &preferences, 3);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(
            recommendations[0].quality,
            crate::domain::Quality::Unfavorable
        );
        assert!(
            recommendations[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("Sin bloque contiguo para 3 personas"))
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

    fn showtime_with_row_layout(id: &str, y: u16, seats: &[(u16, u16, bool)]) -> Showtime {
        Showtime {
            id: id.into(),
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
                seats: seats
                    .iter()
                    .map(|(x, number, available)| Seat {
                        id: format!("G{number}"),
                        row: "G".into(),
                        number: *number,
                        x: *x,
                        y,
                        state: if *available {
                            SeatState::Available
                        } else {
                            SeatState::Occupied
                        },
                    })
                    .collect(),
            },
        }
    }

    fn row_seats(row: &str, y: u16, seats: &[(u16, u16, bool)]) -> Vec<Seat> {
        seats
            .iter()
            .map(|(x, number, available)| Seat {
                id: format!("{row}{number}"),
                row: row.into(),
                number: *number,
                x: *x,
                y,
                state: if *available {
                    SeatState::Available
                } else {
                    SeatState::Occupied
                },
            })
            .collect()
    }
}
