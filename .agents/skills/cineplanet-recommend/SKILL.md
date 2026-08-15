---
name: cineplanet-recommend
description: Find Cineplanet movies, showtimes, and best available seats. Use for natural-language requests about Cineplanet films, cities, venues, formats, earliest availability, contiguous groups, or seat recommendations.
---

# Cineplanet Recommendations

Use the deterministic, noninteractive JSON command only; never drive the TUI.

1. Obtain the required movie, city, and party size (1–5). Ask only for any of these that is missing. Resolve any supplied date in `America/Lima` and pass it as `YYYY-MM-DD` via `--date`. **Date mapping rules (apply verbatim):**
   - Relative dates — `hoy`, `mañana`, `pasado mañana`, `esta noche`, `el lunes`, `este viernes`, etc. — are **always** resolved in `America/Lima` and **always** passed with `--date YYYY-MM-DD`. Never drop them silently; if a user says `hoy`, the command must include `--date`.
   - If the user did **not** request a date, **omit** `--date` so the binary uses its published horizon. Do not invent a default.
2. Apply optional preferences explicitly by repeating the matching flag for each requested value. **Venue-wording rules (apply verbatim — never conflate them):**
   - **Preference language** — `prefiere`, `prefería`, `prioriza`, `favorita`, `favorito`, `ideal`, `queda mejor`, `mejor si es en`, `si hay en`, `si está en` — maps to a repeatable `--favorite-venue`. The venue is a ranking boost, not a hard filter: other venues may still appear in the output.
   - **Strict language** — `solo`, `solamente`, `únicamente`, `exclusivamente`, `en esta sede`, `sí o sí`, `forzosamente`, `ni modo` (used strictly), `que sea en` — maps to `--venue`, which **excludes** every other venue.
   - **Never** convert a preference into a hard venue filter. Passing `--venue` when the user said `prefiere` wipes out every other venue and silently changes the answer; this is the bug we are fixing.
   - Repeat `--date`, `--language`, `--format`, `--room-type`, or `--favorite-venue` for each requested value. Use `--limit` when the user asks for a number of options. Invoke once whenever possible.
3. Pick the executable from the current directory. The repo-local build is the primary path; an installed `cineplanet-cli` is allowed only outside the checkout. Outside the checkout, first verify the binary exists, then probe it with `cineplanet-cli recommend --help` — never a generic `help`/`--help`, because stale pre-clap binaries interpret that as launching the TUI and can panic without a TTY. If the probe fails, do not retry as a network error — surface a clear message and ask for the repository path or a (re)install:

   ```bash
   if repo_root="$(git -C . rev-parse --show-toplevel 2>/dev/null)" \
      && [ -f "${repo_root}/Cargo.toml" ] \
      && [ -f "${repo_root}/.agents/skills/cineplanet-recommend/SKILL.md" ]; then
     cargo run --quiet --manifest-path "${repo_root}/Cargo.toml" -- recommend --movie-title "..." --city "..." --party-size 2
   elif command -v cineplanet-cli >/dev/null 2>&1 \
      && cineplanet-cli recommend --help >/dev/null 2>&1; then
     cineplanet-cli recommend --movie-title "..." --city "..." --party-size 2
   else
     echo "Cannot resolve CineplanetCLI: not inside the checkout ($(pwd)) and the installed cineplanet-cli is missing or does not expose 'recommend'." >&2
     echo "Provide the repository path (for cargo run) or (re)install cineplanet-cli." >&2
     exit 1
   fi
   ```

   Resolve the repository from the current working directory so this works no matter which subdirectory you are in. If `git rev-parse` fails, the probe-or-install branch still runs. Do not guess or fall back to `$HOME` for the repository path; if neither path resolves, ask the user for the absolute path.
4. Parse the JSON response. Require `version` to be `v1`. Trust its `recommendations` order, `viewing.score`, `viewing.zone`, reason codes, and selected block; never recompute rankings from seat maps. Every successful response also includes a top-level `observed_at` (RFC 3339, UTC); surface that timestamp to the user as the freshness of the recommendation, e.g. "observado a las 2026-08-15T12:34:56+00:00".
5. State that availability was observed for this query and can change. Quote the `observed_at` value when describing how fresh the result is, and re-run the command if the user asks about anything more recent. Report any `diagnostics.map_failures` as partial map failures; report an empty result rather than inventing an alternative. Surface the versioned JSON error envelope on failure. Note that error envelopes intentionally do not include `observed_at` because they do not represent observed availability.

This skill is read-only. Never log in, reserve, hold seats, or buy tickets.
