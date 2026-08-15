//! Preference-aware deterministic ordering of pure viewing assessments.

use crate::{
    domain::{Preferences, Recommendation, SeatingArrangement, Showtime},
    viewing,
};

pub fn analyze_showtime(showtime: &Showtime, preferences: &Preferences) -> Option<Recommendation> {
    let assessment = viewing::assess(showtime, preferences.party_size)?;
    let favorite = preferences.favorite_venue_ids.contains(&showtime.venue_id);
    let mut reasons = assessment.reasons;
    if favorite {
        reasons.push("Sede favorita".into());
    }
    Some(Recommendation {
        showtime: showtime.clone(),
        block: assessment.block,
        arrangement: assessment.arrangement,
        quality: assessment.quality,
        reasons,
        // Ranking score deliberately adds preferences; viewing::visual_score never does.
        score: assessment.visual_score + if favorite { 8.0 } else { 0.0 },
        visual_score: assessment.visual_score,
    })
}

pub fn recommend(
    showtimes: &[Showtime],
    preferences: &Preferences,
    limit: usize,
) -> Vec<Recommendation> {
    if preferences.party_size == 0 || limit == 0 {
        return Vec::new();
    }
    let mut results: Vec<_> = showtimes
        .iter()
        .filter(|showtime| acceptable(showtime, preferences))
        .filter_map(|showtime| analyze_showtime(showtime, preferences))
        .collect();
    let has_apt_across_aisle = results.iter().any(|result| {
        result.quality != crate::domain::Quality::Unfavorable
            && matches!(result.arrangement, SeatingArrangement::AcrossAisle { .. })
    });
    let has_apt = results
        .iter()
        .any(|result| result.quality != crate::domain::Quality::Unfavorable);
    if has_apt {
        results.retain(|result| {
            result.quality != crate::domain::Quality::Unfavorable
                || (has_apt_across_aisle && result.arrangement == SeatingArrangement::Together)
        });
    }
    results.sort_by(|left, right| {
        arrangement_priority(&left.arrangement)
            .cmp(&arrangement_priority(&right.arrangement))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.showtime.starts_at.cmp(&right.showtime.starts_at))
            .then_with(|| left.showtime.venue_id.cmp(&right.showtime.venue_id))
            .then_with(|| left.showtime.id.cmp(&right.showtime.id))
    });
    results.truncate(limit);
    results
}

fn acceptable(showtime: &Showtime, preferences: &Preferences) -> bool {
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
}

fn arrangement_priority(arrangement: &SeatingArrangement) -> u8 {
    match arrangement {
        SeatingArrangement::Together => 0,
        SeatingArrangement::AcrossAisle { .. } => 1,
        SeatingArrangement::Scattered => 2,
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::{analyze_showtime, recommend};
    use crate::domain::{
        Modality, Preferences, Quality, Seat, SeatMap, SeatState, SeatingArrangement, Showtime,
    };

    fn show(id: &str, y: u16, layout: &[(u16, u16, bool)]) -> Showtime {
        Showtime {
            id: id.into(),
            movie_id: "movie".into(),
            movie_title: "Movie".into(),
            venue_id: "venue".into(),
            venue_name: "Venue".into(),
            starts_at: DateTime::parse_from_rfc3339("2026-08-10T20:30:00-05:00").unwrap(),
            modality: Modality {
                projection_format: "2D".into(),
                language: "Subtitulada".into(),
                room_type: "Regular".into(),
            },
            seat_map: SeatMap {
                rows: 10,
                columns: 10,
                seats: layout
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

    fn block(id: &str, y: u16, x: u16) -> Showtime {
        let mut seats = Vec::new();
        for row in 0..10 {
            for column in 0..10 {
                seats.push((
                    column,
                    column + 1,
                    row == y && (column == x || column == x + 1),
                ));
            }
        }
        show(id, y, &seats)
    }

    fn party(size: usize) -> Preferences {
        Preferences {
            party_size: size,
            ..Preferences::default()
        }
    }
    fn ids(result: &crate::domain::Recommendation) -> Vec<&str> {
        result.block.iter().map(|seat| seat.id.as_str()).collect()
    }

    #[test]
    fn recommends_the_contiguous_block_closest_to_the_ideal_viewing_point() {
        let mut ideal = block("ideal", 6, 4);
        ideal
            .seat_map
            .seats
            .extend(show("far", 9, &[(0, 1, true), (1, 2, true)]).seat_map.seats);
        assert_eq!(ids(&recommend(&[ideal], &party(2), 1)[0]), ["G5", "G6"]);
    }
    #[test]
    fn recommends_a_contiguous_block_with_mirrored_x_coordinates() {
        let mut value = block("mirror", 6, 4);
        for seat in &mut value.seat_map.seats {
            seat.x = 9 - seat.x;
        }
        assert_eq!(ids(&recommend(&[value], &party(2), 1)[0]), ["G5", "G6"]);
    }
    #[test]
    fn recommends_four_plus_one_across_a_physical_aisle() {
        let result = analyze_showtime(
            &show(
                "a",
                6,
                &[
                    (0, 1, true),
                    (1, 2, true),
                    (2, 3, true),
                    (3, 4, true),
                    (5, 5, true),
                ],
            ),
            &party(5),
        )
        .unwrap();
        assert_eq!(
            result.arrangement,
            SeatingArrangement::AcrossAisle {
                first: 4,
                second: 1
            }
        );
    }
    #[test]
    fn prefers_four_plus_one_over_three_plus_two_across_the_same_aisle() {
        let result = analyze_showtime(
            &show(
                "a",
                6,
                &[
                    (0, 1, true),
                    (1, 2, true),
                    (2, 3, true),
                    (3, 4, true),
                    (5, 5, true),
                    (6, 6, true),
                ],
            ),
            &party(5),
        )
        .unwrap();
        assert_eq!(
            result.arrangement,
            SeatingArrangement::AcrossAisle {
                first: 4,
                second: 1
            }
        );
    }
    #[test]
    fn does_not_treat_an_occupied_seat_as_an_aisle() {
        let result = analyze_showtime(
            &show(
                "a",
                6,
                &[
                    (0, 1, true),
                    (1, 2, true),
                    (2, 3, true),
                    (3, 4, true),
                    (4, 5, false),
                    (5, 6, true),
                ],
            ),
            &party(5),
        )
        .unwrap();
        assert_eq!(result.arrangement, SeatingArrangement::Scattered);
    }
    #[test]
    fn requires_both_aisle_blocks_to_be_in_the_same_row() {
        let mut value = show(
            "a",
            6,
            &[(0, 1, true), (1, 2, true), (2, 3, true), (3, 4, true)],
        );
        value.seat_map.seats.push(Seat {
            id: "H5".into(),
            row: "H".into(),
            number: 5,
            x: 5,
            y: 7,
            state: SeatState::Available,
        });
        assert_eq!(
            analyze_showtime(&value, &party(5)).unwrap().arrangement,
            SeatingArrangement::Scattered
        );
    }
    #[test]
    fn recommends_across_an_aisle_with_mirrored_x_coordinates() {
        let result = analyze_showtime(
            &show(
                "a",
                6,
                &[
                    (9, 1, true),
                    (8, 2, true),
                    (7, 3, true),
                    (6, 4, true),
                    (4, 5, true),
                ],
            ),
            &party(5),
        )
        .unwrap();
        assert!(matches!(
            result.arrangement,
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
        let result = analyze_showtime(
            &show(
                "a",
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
            ),
            &party(5),
        )
        .unwrap();
        assert_eq!(result.arrangement, SeatingArrangement::Together);
    }
    #[test]
    fn ranks_a_contiguous_block_before_a_better_positioned_aisle_option() {
        let together = show(
            "t",
            2,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, true),
            ],
        );
        let aisle = show(
            "a",
            6,
            &[
                (3, 1, true),
                (4, 2, true),
                (5, 3, true),
                (6, 4, true),
                (8, 5, true),
            ],
        );
        assert_eq!(
            recommend(&[aisle, together], &party(5), 3)[0].showtime.id,
            "t"
        );
    }
    #[test]
    fn continues_past_an_oversized_front_row_run_to_find_a_valid_aisle_row() {
        let mut value = show(
            "a",
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
        for seat in &mut value.seat_map.seats {
            seat.row = "A".into();
        }
        value.seat_map.seats.extend(
            show(
                "g",
                6,
                &[
                    (0, 1, true),
                    (1, 2, true),
                    (2, 3, true),
                    (3, 4, true),
                    (5, 5, true),
                ],
            )
            .seat_map
            .seats,
        );
        assert!(matches!(
            analyze_showtime(&value, &party(5)).unwrap().arrangement,
            SeatingArrangement::AcrossAisle { .. }
        ));
    }
    #[test]
    fn continues_to_a_later_physical_aisle_when_the_first_cannot_fit_the_group() {
        let result = analyze_showtime(
            &show(
                "a",
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
            ),
            &party(5),
        )
        .unwrap();
        assert_eq!(
            result.arrangement,
            SeatingArrangement::AcrossAisle {
                first: 2,
                second: 3
            }
        );
    }
    #[test]
    fn ranks_an_unfavorable_across_aisle_option_before_scattered_seats() {
        let across = show(
            "a",
            9,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (5, 5, true),
            ],
        );
        let scattered = show("s", 6, &[(0, 1, true), (2, 2, true), (4, 3, true)]);
        assert_eq!(
            recommend(&[scattered, across], &party(5), 3)[0].showtime.id,
            "a"
        );
    }
    #[test]
    fn keeps_an_unfavorable_together_option_when_an_across_aisle_option_is_apt() {
        let together = show(
            "t",
            9,
            &[
                (0, 1, true),
                (1, 2, true),
                (2, 3, true),
                (3, 4, true),
                (4, 5, true),
            ],
        );
        let across = show(
            "a",
            6,
            &[
                (3, 1, true),
                (4, 2, true),
                (5, 3, true),
                (6, 4, true),
                (8, 5, true),
            ],
        );
        assert_eq!(recommend(&[across, together], &party(5), 3).len(), 2);
    }
    #[test]
    fn excludes_showtimes_in_languages_the_user_did_not_accept() {
        let mut prefs = party(2);
        prefs.accepted_languages.insert("Subtitulada".into());
        let mut dubbed = block("d", 6, 4);
        dubbed.modality.language = "Doblada".into();
        assert_eq!(
            recommend(&[dubbed, block("s", 6, 4)], &prefs, 3)[0]
                .showtime
                .id,
            "s"
        );
    }
    #[test]
    fn prefers_a_favorite_venue_when_seat_quality_is_close() {
        let mut prefs = party(2);
        prefs.favorite_venue_ids.insert("fav".into());
        let ideal = block("ideal", 6, 4);
        let mut favorite = block("fav", 7, 4);
        favorite.venue_id = "fav".into();
        assert_eq!(
            recommend(&[ideal, favorite], &prefs, 3)[0].showtime.id,
            "fav"
        );
    }
    #[test]
    fn excludes_projection_formats_and_room_types_the_user_did_not_accept() {
        let mut prefs = party(2);
        prefs.accepted_formats.insert("2D".into());
        prefs.accepted_room_types.insert("Regular".into());
        let mut three_d = block("3d", 6, 4);
        three_d.modality.projection_format = "3D".into();
        let mut prime = block("p", 6, 4);
        prime.modality.room_type = "Prime".into();
        assert_eq!(
            recommend(&[three_d, prime, block("ok", 6, 4)], &prefs, 3)[0]
                .showtime
                .id,
            "ok"
        );
    }
    #[test]
    fn hides_unfavorable_alternatives_when_an_apt_showtime_exists() {
        assert_eq!(
            recommend(&[block("bad", 9, 0), block("apt", 6, 4)], &party(2), 3).len(),
            1
        );
    }
    #[test]
    fn analyzes_an_unfavorable_showtime_even_when_recommend_hides_it() {
        let value = block("bad", 9, 0);
        assert_eq!(
            analyze_showtime(&value, &party(2)).unwrap().quality,
            Quality::Unfavorable
        );
    }
    #[test]
    fn does_not_analyze_a_zero_person_party() {
        assert!(analyze_showtime(&block("a", 6, 4), &party(0)).is_none());
    }
    #[test]
    fn returns_clearly_labeled_alternatives_when_no_apt_showtime_exists() {
        assert_eq!(
            recommend(&[block("bad", 9, 0)], &party(2), 3)[0].quality,
            Quality::Unfavorable
        );
    }
    #[test]
    fn keeps_available_showtimes_when_the_group_has_no_contiguous_block() {
        let result = recommend(&[block("split", 6, 4)], &party(3), 3);
        assert_eq!(result[0].quality, Quality::Unfavorable);
        assert!(result[0].reasons[0].contains("Sin bloque contiguo para 3 personas"));
    }
    #[test]
    fn better_seats_can_outrank_a_favorite_venue() {
        let mut prefs = party(2);
        prefs.favorite_venue_ids.insert("fav".into());
        let mut favorite = block("fav", 9, 0);
        favorite.venue_id = "fav".into();
        assert_eq!(
            recommend(&[favorite, block("better", 6, 4)], &prefs, 3)[0]
                .showtime
                .id,
            "better"
        );
    }
}
