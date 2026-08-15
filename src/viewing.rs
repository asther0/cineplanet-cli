//! Pure seat-map geometry and viewing-quality assessment.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::{Quality, Seat, SeatMap, SeatState, SeatingArrangement, Showtime};

/// The lowest visual score considered a good viewing position.  Ranking and
/// presentation deliberately share this boundary.
pub const GOOD_VIEWING_SCORE_THRESHOLD: f64 = 72.0;
pub const GOOD_VIEWING_ZONE_ID: &str = "central_middle_rear";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewingZoneSpan {
    pub y: u16,
    pub start_x: u16,
    pub end_x: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewingZone {
    pub score_threshold: f64,
    pub spans: Vec<ViewingZoneSpan>,
}

impl ViewingZone {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.spans
            .iter()
            .any(|span| span.y == y && (span.start_x..=span.end_x).contains(&x))
    }
}

pub fn selected_block_is_in_good_viewing_zone(
    map: &SeatMap,
    block: &[Seat],
    arrangement: &SeatingArrangement,
) -> bool {
    if matches!(arrangement, SeatingArrangement::Scattered) {
        return false;
    }
    let zone = good_viewing_zone(map);
    block.iter().all(|seat| zone.contains(seat.x, seat.y))
}

/// Computes the complete geometry of positions with a good viewing score.
/// It intentionally uses the map dimensions rather than available seats, so a
/// sold seat does not punch a misleading hole through the viewing zone.
pub fn good_viewing_zone(map: &SeatMap) -> ViewingZone {
    let mut spans = Vec::new();
    for y in 0..map.rows {
        let mut start = None;
        for x in 0..map.columns {
            let good = position_score(map, x, y)
                .is_some_and(|score| score >= GOOD_VIEWING_SCORE_THRESHOLD);
            match (start, good) {
                (None, true) => start = Some(x),
                (Some(start_x), false) => {
                    spans.push(ViewingZoneSpan {
                        y,
                        start_x,
                        end_x: x - 1,
                    });
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(start_x) = start {
            spans.push(ViewingZoneSpan {
                y,
                start_x,
                end_x: map.columns.saturating_sub(1),
            });
        }
    }
    ViewingZone {
        score_threshold: GOOD_VIEWING_SCORE_THRESHOLD,
        spans,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewingAssessment {
    pub block: Vec<Seat>,
    pub arrangement: SeatingArrangement,
    pub quality: Quality,
    pub visual_score: f64,
    pub reason_codes: Vec<&'static str>,
    pub reasons: Vec<String>,
}

pub fn assess(showtime: &Showtime, party_size: usize) -> Option<ViewingAssessment> {
    if party_size == 0 {
        return None;
    }
    let mut by_row: BTreeMap<&str, Vec<&Seat>> = BTreeMap::new();
    for seat in &showtime.seat_map.seats {
        if seat.state == SeatState::Available {
            by_row.entry(&seat.row).or_default().push(seat);
        }
    }

    let mut candidates = Vec::new();
    for seats in by_row.values_mut() {
        seats.sort_by_key(|seat| (seat.x, seat.number, &seat.id));
        for window in seats.windows(party_size) {
            if window.windows(2).all(|pair| contiguous(pair[0], pair[1])) {
                let Some(visual_score) = score(showtime, window) else {
                    continue;
                };
                let mut block = window
                    .iter()
                    .map(|seat| (*seat).clone())
                    .collect::<Vec<_>>();
                block.sort_by_key(|seat| seat.number);
                candidates.push((visual_score, block, SeatingArrangement::Together));
            }
        }
    }
    candidates.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| block_key(&left.1).cmp(&block_key(&right.1)))
    });
    if let Some((visual_score, block, arrangement)) = candidates.into_iter().next() {
        return Some(finish(block, arrangement, visual_score));
    }

    if let Some((visual_score, block, first, second)) = across_aisle(showtime, party_size) {
        return Some(finish(
            block,
            SeatingArrangement::AcrossAisle { first, second },
            visual_score,
        ));
    }

    let available: Vec<_> = showtime
        .seat_map
        .seats
        .iter()
        .filter(|seat| seat.state == SeatState::Available)
        .cloned()
        .collect();
    let representative = available.first()?.clone();
    Some(ViewingAssessment {
        block: vec![representative],
        arrangement: SeatingArrangement::Scattered,
        quality: Quality::Unfavorable,
        visual_score: 0.0,
        reason_codes: vec!["no_contiguous_block"],
        reasons: vec![format!(
            "Sin bloque contiguo para {party_size} personas; quedan {} asientos disponibles",
            available.len()
        )],
    })
}

fn finish(
    block: Vec<Seat>,
    arrangement: SeatingArrangement,
    visual_score: f64,
) -> ViewingAssessment {
    let quality = if visual_score >= 90.0 {
        Quality::Excellent
    } else if visual_score >= GOOD_VIEWING_SCORE_THRESHOLD {
        Quality::Good
    } else {
        Quality::Unfavorable
    };
    let (reason_code, reason) = match arrangement {
        SeatingArrangement::Together => (
            "contiguous_central_block",
            "Bloque contiguo cerca del centro de la sala",
        ),
        SeatingArrangement::AcrossAisle { .. } => (
            "across_aisle_block",
            "Dos bloques contiguos separados por un pasillo",
        ),
        SeatingArrangement::Scattered => ("no_contiguous_block", "Sin bloque contiguo"),
    };
    ViewingAssessment {
        block,
        arrangement,
        quality,
        visual_score,
        reason_codes: vec![reason_code],
        reasons: vec![reason.into()],
    }
}

fn contiguous(left: &Seat, right: &Seat) -> bool {
    left.x.abs_diff(right.x) == 1 && left.number.abs_diff(right.number) == 1
}

fn score(showtime: &Showtime, block: &[&Seat]) -> Option<f64> {
    let center_x = block.iter().map(|seat| f64::from(seat.x)).sum::<f64>() / block.len() as f64;
    let center_y = block.iter().map(|seat| f64::from(seat.y)).sum::<f64>() / block.len() as f64;
    let x = center_x / f64::from(showtime.seat_map.columns.saturating_sub(1).max(1));
    let y = center_y / f64::from(showtime.seat_map.rows.saturating_sub(1).max(1));
    score_at_normalized(x, y)
}

fn position_score(map: &SeatMap, x: u16, y: u16) -> Option<f64> {
    let x = f64::from(x) / f64::from(map.columns.saturating_sub(1).max(1));
    let y = f64::from(y) / f64::from(map.rows.saturating_sub(1).max(1));
    score_at_normalized(x, y)
}

fn score_at_normalized(x: f64, y: f64) -> Option<f64> {
    (y >= 0.20).then(|| 100.0 - (x - 0.5).abs() * 70.0 - (y - 0.66).abs() * 45.0)
}

fn block_key(block: &[Seat]) -> Vec<(&str, u16, u16, u16)> {
    block
        .iter()
        .map(|seat| (seat.row.as_str(), seat.number, seat.x, seat.y))
        .collect()
}

fn across_aisle(showtime: &Showtime, party_size: usize) -> Option<(f64, Vec<Seat>, usize, usize)> {
    let mut by_row: BTreeMap<&str, Vec<&Seat>> = BTreeMap::new();
    for seat in &showtime.seat_map.seats {
        by_row.entry(&seat.row).or_default().push(seat);
    }
    let mut candidates = Vec::new();
    for seats in by_row.values_mut() {
        seats.sort_by_key(|seat| (seat.x, seat.number));
        for pair in seats.windows(2).filter(|pair| pair[1].x > pair[0].x + 1) {
            let left = run_ending(seats, pair[0].x);
            let right = run_starting(seats, pair[1].x);
            for first in 1..=left.len().min(party_size.saturating_sub(1)) {
                let second = party_size - first;
                if second == 0 || second > right.len() {
                    continue;
                }
                let mut block: Vec<Seat> = left[left.len() - first..]
                    .iter()
                    .chain(right[..second].iter())
                    .map(|seat| (*seat).clone())
                    .collect();
                block.sort_by_key(|seat| (seat.x, seat.number));
                let references: Vec<_> = block.iter().collect();
                if let Some(visual_score) = score(showtime, &references) {
                    candidates.push((first.min(second), visual_score, block, first, second));
                }
            }
        }
    }
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.total_cmp(&left.1))
            .then_with(|| block_key(&left.2).cmp(&block_key(&right.2)))
    });
    candidates
        .into_iter()
        .next()
        .map(|(_, score, block, first, second)| (score, block, first, second))
}

fn run_ending<'a>(seats: &[&'a Seat], x: u16) -> Vec<&'a Seat> {
    let Some(end) = seats.iter().position(|seat| seat.x == x) else {
        return Vec::new();
    };
    // The run endpoint must itself be Available; otherwise the seat across
    // the aisle has no neighbor to pair with.
    if seats[end].state != SeatState::Available {
        return Vec::new();
    }
    // Walk left only across seats that are both geometrically contiguous and
    // Available. Any unavailable seat splits the run, so the returned slice
    // is guaranteed to contain only Available seats.
    let mut start = end;
    while start > 0
        && seats[start - 1].state == SeatState::Available
        && contiguous(seats[start - 1], seats[start])
    {
        start -= 1;
    }
    seats[start..=end].to_vec()
}

fn run_starting<'a>(seats: &[&'a Seat], x: u16) -> Vec<&'a Seat> {
    let Some(start) = seats.iter().position(|seat| seat.x == x) else {
        return Vec::new();
    };
    if seats[start].state != SeatState::Available {
        return Vec::new();
    }
    let mut end = start;
    while end + 1 < seats.len()
        && seats[end + 1].state == SeatState::Available
        && contiguous(seats[end], seats[end + 1])
    {
        end += 1;
    }
    seats[start..=end].to_vec()
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::domain::{Modality, SeatMap};

    fn seat(id: &str, x: u16, state: SeatState) -> Seat {
        Seat {
            id: id.into(),
            row: "G".into(),
            number: x + 1,
            x,
            y: 6,
            state,
        }
    }

    fn showtime_with(seats: Vec<Seat>) -> Showtime {
        Showtime {
            id: "test".into(),
            movie_id: "movie".into(),
            movie_title: "Movie".into(),
            movie_details_url: None,
            venue_id: "venue".into(),
            venue_name: "Venue".into(),
            session_id: None,
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
        }
    }

    #[test]
    fn accessible_seats_do_not_complete_a_contiguous_block() {
        let showtime = showtime_with(vec![
            seat("G5", 4, SeatState::Available),
            seat("G6", 5, SeatState::Accessible),
        ]);
        let assessment = assess(&showtime, 2).unwrap();
        assert_eq!(assessment.arrangement, SeatingArrangement::Scattered);
    }

    #[test]
    fn good_viewing_zone_uses_the_same_good_threshold_as_assessment() {
        let map = SeatMap {
            rows: 10,
            columns: 10,
            seats: Vec::new(),
        };

        let zone = good_viewing_zone(&map);

        assert_eq!(zone.score_threshold, GOOD_VIEWING_SCORE_THRESHOLD);
        assert!(zone.contains(4, 6));
        assert!(!zone.contains(0, 6));
        assert!(!zone.contains(4, 0));
    }

    #[test]
    fn run_ending_stops_at_an_occupied_intermediate_seat() {
        // Geometry-only contiguous walks used to include G2 (Occupied) in the
        // run ending at x=4 because contiguous() only checks x/number. The
        // fix must stop the walk at the unavailable seat.
        let seats = [
            seat("G1", 0, SeatState::Available),
            seat("G2", 1, SeatState::Occupied),
            seat("G3", 2, SeatState::Available),
            seat("G4", 3, SeatState::Available),
            seat("G5", 4, SeatState::Available),
        ];
        let refs: Vec<&Seat> = seats.iter().collect();
        let run = run_ending(&refs, 4);
        let ids: Vec<&str> = run.iter().map(|seat| seat.id.as_str()).collect();
        assert_eq!(ids, vec!["G3", "G4", "G5"]);
    }

    #[test]
    fn run_starting_stops_at_an_accessible_intermediate_seat() {
        let seats = [
            seat("G6", 6, SeatState::Available),
            seat("G7", 7, SeatState::Available),
            seat("G8", 8, SeatState::Accessible),
            seat("G9", 9, SeatState::Available),
        ];
        let refs: Vec<&Seat> = seats.iter().collect();
        let run = run_starting(&refs, 6);
        let ids: Vec<&str> = run.iter().map(|seat| seat.id.as_str()).collect();
        assert_eq!(ids, vec!["G6", "G7"]);
    }

    #[test]
    fn across_aisle_assessment_excludes_an_occupied_intermediate_seat() {
        // Party of 5 split by an aisle at x=5: 4 available seats on the left
        // (x=0..3) and 3 on the right (x=6..8). With an occupied seat at x=1
        // the left run is only the contiguous available segment x=2..3, so
        // there is no across-aisle block that fits 5 — and crucially the
        // occupied seat must never leak into the result.
        let showtime = showtime_with(vec![
            seat("G1", 0, SeatState::Available),
            seat("G2", 1, SeatState::Occupied),
            seat("G3", 2, SeatState::Available),
            seat("G4", 3, SeatState::Available),
            seat("G5", 4, SeatState::Available),
            seat("G6", 6, SeatState::Available),
            seat("G7", 7, SeatState::Available),
            seat("G8", 8, SeatState::Available),
        ]);
        let assessment = assess(&showtime, 5).unwrap();
        for seat in &assessment.block {
            assert_eq!(
                seat.state,
                SeatState::Available,
                "across-aisle block must not contain unavailable seats, got {} ({:?})",
                seat.id,
                seat.state
            );
        }
    }

    #[test]
    fn across_aisle_assessment_excludes_an_accessible_intermediate_seat() {
        // Same shape as the occupied test, but the splitter is accessible so
        // an honest run also stops at it.
        let showtime = showtime_with(vec![
            seat("G1", 0, SeatState::Available),
            seat("G2", 1, SeatState::Accessible),
            seat("G3", 2, SeatState::Available),
            seat("G4", 3, SeatState::Available),
            seat("G5", 4, SeatState::Available),
            seat("G6", 6, SeatState::Available),
            seat("G7", 7, SeatState::Available),
            seat("G8", 8, SeatState::Available),
        ]);
        let assessment = assess(&showtime, 5).unwrap();
        for seat in &assessment.block {
            assert_eq!(
                seat.state,
                SeatState::Available,
                "across-aisle block must not contain unavailable seats, got {} ({:?})",
                seat.id,
                seat.state
            );
        }
    }
}
