use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cineplanet-cli"))
}

fn recommend_demo(args: &[&str]) -> serde_json::Value {
    let output = binary()
        .env("CINEPLANET_DEMO", "1")
        .arg("recommend")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn recommend_in_demo_returns_a_versioned_json_contract_with_a_compact_seat_preview() {
    let output = binary()
        .env("CINEPLANET_DEMO", "1")
        .args([
            "recommend",
            "--movie-title",
            "la odisea",
            "--city",
            "lima",
            "--party-size",
            "2",
            "--limit",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["version"], "v1");
    assert_eq!(json["query"]["movie_title"], "La Odisea");
    assert!(json["recommendations"][0].get("seat_map").is_none());
    let preview = &json["recommendations"][0]["seat_preview"];
    assert_eq!(preview["screen"], "PANTALLA");
    assert_eq!(preview["symbols"]["available"], ".");
    assert_eq!(preview["symbols"]["occupied"], "#");
    assert_eq!(preview["symbols"]["accessible"], "A");
    assert_eq!(preview["symbols"]["recommended"], "*");
    assert_eq!(preview["symbols"]["aisle"], " ");
    let rows = preview["rows"].as_array().unwrap();
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .any(|row| row["layout"].as_str().unwrap().contains('*'))
    );
    assert!(
        rows.iter()
            .all(|row| row["layout"].as_str().unwrap().is_ascii())
    );
    assert!(
        json["recommendations"][0].get("checkout_handoff").is_none(),
        "demo data must remain valid even when no official checkout URL exists"
    );
    assert!(json["recommendations"][0]["selected_block"].is_object());
    assert_eq!(
        json["recommendations"][0]["viewing"]["zone"]["id"],
        "central_middle_rear"
    );
    assert_eq!(
        json["recommendations"][0]["viewing"]["zone"]["selected_block_inside"],
        true
    );
}

#[test]
fn help_is_ordinary_stdout_output_with_a_success_status() {
    for args in [["--help"].as_slice(), ["recommend", "--help"].as_slice()] {
        let output = binary().args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
}

#[test]
fn recommend_returns_a_structured_not_found_error() {
    let output = binary()
        .env("CINEPLANET_DEMO", "1")
        .args([
            "recommend",
            "--movie-title",
            "missing",
            "--city",
            "Lima",
            "--party-size",
            "2",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(json["version"], "v1");
    assert_eq!(json["error"]["kind"], "movie_not_found");
}

#[test]
fn recommend_with_filters_and_no_matches_succeeds_with_an_empty_json_result() {
    let output = binary()
        .env("CINEPLANET_DEMO", "1")
        .args([
            "recommend",
            "--movie-id",
            "odyssey",
            "--city",
            "Lima",
            "--party-size",
            "2",
            "--venue",
            "CP La Molina",
            "--language",
            "Subtitulada",
            "--format",
            "2D",
            "--room-type",
            "Regular",
            "--date",
            "1900-01-01",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["version"], "v1");
    assert_eq!(json["recommendations"], serde_json::json!([]));
    assert_eq!(json["diagnostics"]["candidate_count"], 0);
}

#[test]
fn invalid_recommend_arguments_use_the_stderr_error_envelope() {
    let output = binary()
        .args([
            "recommend",
            "--movie-id",
            "odyssey",
            "--city",
            "Lima",
            "--party-size",
            "0",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(json["version"], "v1");
    assert_eq!(json["error"]["kind"], "recommend_failed");
    assert_eq!(json["error"]["stage"], "arguments");
}

#[test]
fn favorite_venue_changes_ranking_but_not_the_reported_visual_score() {
    let args = [
        "recommend",
        "--movie-id",
        "odyssey",
        "--city",
        "Lima",
        "--party-size",
        "2",
        "--venue",
        "la-molina",
    ];
    let plain = binary()
        .env("CINEPLANET_DEMO", "1")
        .args(args)
        .output()
        .unwrap();
    let favorite = binary()
        .env("CINEPLANET_DEMO", "1")
        .args(args)
        .args(["--favorite-venue", "la-molina"])
        .output()
        .unwrap();
    let plain: serde_json::Value = serde_json::from_slice(&plain.stdout).unwrap();
    let favorite: serde_json::Value = serde_json::from_slice(&favorite.stdout).unwrap();
    assert_eq!(
        plain["recommendations"][0]["viewing"]["score"],
        favorite["recommendations"][0]["viewing"]["score"]
    );
    assert_eq!(
        favorite["recommendations"][0]["viewing"]["reason_codes"],
        serde_json::json!(["contiguous_central_block", "favorite_venue"])
    );
}

#[test]
fn recommend_response_observed_at_is_present_and_rfc3339_parseable() {
    let json = recommend_demo(&[
        "--movie-title",
        "la odisea",
        "--city",
        "lima",
        "--party-size",
        "2",
        "--limit",
        "1",
    ]);

    let observed_at = json["observed_at"]
        .as_str()
        .expect("observed_at must serialize as a JSON string");
    let parsed = chrono::DateTime::parse_from_rfc3339(observed_at)
        .expect("observed_at must be parseable as RFC 3339");
    assert_eq!(
        parsed.offset().local_minus_utc(),
        0,
        "observed_at must be reported in UTC (offset 0)"
    );
    assert_eq!(json["version"], "v1");
}

#[test]
fn recommend_error_envelope_does_not_include_observed_at() {
    let output = binary()
        .env("CINEPLANET_DEMO", "1")
        .args([
            "recommend",
            "--movie-title",
            "missing",
            "--city",
            "Lima",
            "--party-size",
            "2",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        json.get("observed_at").is_none(),
        "error envelopes must remain free of freshness metadata: {json}"
    );
}
