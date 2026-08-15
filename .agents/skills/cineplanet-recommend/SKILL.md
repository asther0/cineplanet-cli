---
name: cineplanet-recommend
description: Find Cineplanet movies, showtimes, and best available seats. Use for natural-language requests about Cineplanet films, cities, venues, formats, earliest availability, contiguous groups, or seat recommendations.
---

# Cineplanet Recommendations

Use the deterministic, noninteractive JSON command only; never drive the TUI.

1. Obtain the required movie, city, and party size (1–5). Ask only for any of these that is missing. Translate any supplied date to `America/Lima` and pass it as `YYYY-MM-DD`.
2. Apply optional preferences explicitly: repeat `--date`, `--venue`, `--language`, `--format`, `--room-type`, or `--favorite-venue` for each requested value. Use `--limit` when the user asks for a number of options. Invoke once whenever possible.
3. Run the installed binary:

   ```bash
   cineplanet-cli recommend --movie-title "..." --city "..." --party-size 2
   ```

   If `cineplanet-cli` is unavailable, run the same arguments with the repository manifest. Resolve the repository root first so this works no matter which subdirectory you are in, then point `cargo` at its `Cargo.toml`. If `git rev-parse` fails (not inside a working tree, `git` missing, etc.) abort clearly and ask the user to provide the absolute path to the repository checkout; do not guess or fall back to `$HOME`:

   ```bash
   if ! repo_root="$(git -C . rev-parse --show-toplevel 2>/dev/null)"; then
     echo "Could not locate the CineplanetCLI repository from $(pwd)." >&2
     echo "Run this from inside the repo, or pass an explicit --manifest-path." >&2
     exit 1
   fi
   cargo run --manifest-path "${repo_root}/Cargo.toml" -- recommend --movie-title "..." --city "..." --party-size 2
   ```

4. Parse the JSON response. Require `version` to be `v1`. Trust its `recommendations` order, `viewing.score`, `viewing.zone`, reason codes, and selected block; never recompute rankings from seat maps. Every successful response also includes a top-level `observed_at` (RFC 3339, UTC); surface that timestamp to the user as the freshness of the recommendation, e.g. "observado a las 2026-08-15T12:34:56+00:00".
5. State that availability was observed for this query and can change. Quote the `observed_at` value when describing how fresh the result is, and re-run the command if the user asks about anything more recent. Report any `diagnostics.map_failures` as partial map failures; report an empty result rather than inventing an alternative. Surface the versioned JSON error envelope on failure. Note that error envelopes intentionally do not include `observed_at` because they do not represent observed availability.

This skill is read-only. Never log in, reserve, hold seats, or buy tickets.