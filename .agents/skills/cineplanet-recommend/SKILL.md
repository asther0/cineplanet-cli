---
name: cineplanet-recommend
description: Find Cineplanet movies, showtimes, and best available seats. Use for natural-language requests about Cineplanet films, cities, venues, formats, earliest availability, contiguous or aisle-separated groups, or seat recommendations, including numeric 1/2/3 follow-ups to continue, revalidate or hold seats, or open guest checkout.
---

# Cineplanet Recommendations

Use the deterministic, noninteractive JSON command only; never drive the TUI.

1. Obtain the required movie, city, and party size (1–5). Ask only for any of these that is missing. Resolve any supplied date in `America/Lima` and pass it as `YYYY-MM-DD` via `--date`. **Date mapping rules (apply verbatim):**
   - Relative dates — `hoy`, `mañana`, `pasado mañana`, `esta noche`, `el lunes`, `este viernes`, etc. — are **always** resolved in `America/Lima` and **always** passed with `--date YYYY-MM-DD`. Never drop them silently; if a user says `hoy`, the command must include `--date`.
   - If the user did **not** request a date, **omit** `--date` so the binary uses its published horizon. Do not invent a default.
   - **Party-size vs options-count disambiguation (apply verbatim — never conflate them).** A request can independently name a group size and a number of options to compare. Resolve each axis separately and pass it as its own flag:
     - **Options-count phrases** — `los N mejores`, `dame N opciones`, `muéstrame N`, `los primeros N`, `top N`, `N funciones`, `N horarios`, `N opciones`, `N alternativas` — set `--limit N`. They control how many recommendations the CLI returns; they are **never** party size. Treating `top 3` or `dame 3 opciones` as `party-size 3` silently turns a comparison request into a group of three, which is the bug we are fixing.
     - **Single-person phrases** — `para mí`, `voy solo`, `voy sola`, `solo yo`, `individual`, `una persona`, `yo nada más` — set `--party-size 1`. Do not default to 1 in their absence.
     - **Unambiguous multi-person phrases** — `somos tres`, `somos 4`, `para 3 personas`, `para cuatro`, `vamos 3`, `tres asientos juntos`, `4 amigos`, `mi familia de 5`, `en pareja`, `con mi novia`, `con mi esposo`, plus `N espacios`, `N lugares`, `N asientos`, `N butacas`, `N entradas`, `espacio para N`, `lugares para N` and `asientos para N` — set `--party-size` to the named count (2 for the couple phrases `en pareja` / `con mi novia` / `con mi esposo`, since they imply two people). A seat/person semantic makes the number unambiguous: for example, `3 espacios buenos`, `4 entradas` and `para 3` in this seating domain mean party sizes 3, 4 and 3 respectively. Do not ask again for these phrases.
     - **Ask only when group size has no signal** — a bare number with no options-count phrase, group noun, seat/person semantic, or other group wording is insufficient. Ask once. Never infer 2 as a default; never reuse a party size from a previous turn.
   - Ask once only when the request provides no group-size signal. Do not run the command with a guessed party size, and do not reuse a party size from a prior turn. The CLI's `--party-size` is independent of `--limit`; both values appear independently in the compact header so the user can spot any misinterpretation.
2. Apply optional preferences explicitly by repeating the matching flag for each requested value. **Venue-wording rules (apply verbatim — never conflate them):**
   - **Preference language** — `prefiere`, `prefería`, `prioriza`, `favorita`, `favorito`, `ideal`, `queda mejor`, `mejor si es en`, `si hay en`, `si está en` — maps to a repeatable `--favorite-venue`. The venue is a ranking boost, not a hard filter: other venues may still appear in the output.
   - **Strict scope language** — an ordinary location/scope clause that names venues without preference wording is a hard filter. Examples: `en San Miguel o Salaverry`, `en Cineplanet San Miguel y Salaverry`, `de San Miguel`, `entre San Miguel y Salaverry`, `busca en X`, `que sea en X`, plus `solo`, `solamente`, `únicamente`, `exclusivamente`, `en esta sede`, `sí o sí` and `forzosamente`. Map every named venue to a repeated `--venue`; every other venue is excluded. The conjunction `o` means “search either of these venues”, not “prefer these venues”.
   - **Never** convert a preference into a hard venue filter. Passing `--venue` when the user said `prefiere` wipes out every other venue and silently changes the answer; this is the bug we are fixing.
   - **Never** convert an explicit venue scope into favorites. `en San Miguel o Salaverry` must invoke `--venue "San Miguel" --venue "Salaverry"`, never `--favorite-venue`. Before presenting results, verify that every returned recommendation matches one of the strict requested venues. If any recommendation falls outside that set, discard the response and rerun once with the correct repeated `--venue` flags. Never show the leaked venue.
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

   **Run the command yourself in this agent — never delegate.** A `recommend`
   call is a deterministic, read-only JSON query against the local CLI. It is
   not work for `delivery-fast`, `delivery-code`, `discovery`, Herdr, panes,
   sub-agents, or any other worker. Execute the bash command inline in the
   current chat and parse its stdout here. Do not hand the request to a
   pane, do not wrap the task as a "research" request to a coordinator,
   do not return a meta-summary that says another agent handled it. One
   local invocation is the default; long JSON output is processed in the
   same agent by reading it section by section, not by spawning helpers.
   When the bash block returns, this agent — the one writing the user
   reply — renders the full output. There is no second hop.
4. Parse the JSON response. Require `version` to be `v1`. Trust its `recommendations` order, `viewing.score`, `viewing.zone`, reason codes, selected block, and `seat_preview`; never recompute rankings from seat maps.

   Render the response in this exact order. A response is **incomplete** if
   any of these blocks is missing, if a `seat_preview` is omitted, if seats
   are compressed, or if filters that the user did not request are invented.

   **Visual rendering rules (human response only; never JSON).** Preserve the
   ASCII JSON contract verbatim. In the map, use the fixed-width, human-only
   palette: `.` → `◻ ` (disponible), `#` → `◼ ` (ocupado), `A` → `♿`,
   `*` → `♟︎ ` (recomendado), and a JSON space → two spaces
   (pasillo/hueco). Always use the text-presentation chess pawn `♟︎` as a
   small person-like marker; do not depend on foreground color. It must remain
   visually distinct from both available `◻` and occupied `◼` cells in a
   monochrome client.
   Never use the `🟧` emoji: macOS renders it larger than the other cells and
   with a gradient.
   Do not use
   borders around maps or right-side borders: emoji widths make them unreliable.
   Do not mix raw ASCII map cells with this palette.

   a. **Cabecera compacta** — replace the dense `Tu búsqueda` and `Resumen`
      paragraphs with exactly three short visual lines. Keep Personas and
      Opciones visibly independent. Do not print a separate summary or an
      `Opciones progresivas` heading:

      ```text
      🎬 PELÍCULA · FECHA/HORIZONTE · HORARIO · N PERSONAS
      📍 SOLO SEDE 1 + SEDE 2 · N OPCIONES
      🕒 Actualizado HH:MM (Lima) · X/Y mapas · Z agotadas/fallas
      ```

      - **Película** — title exactly as quoted.
      - **Fecha** — `YYYY-MM-DD` in `America/Lima`, or `(no pedido)`.
      - **Horario** — the requested time window, or `cualquier horario
        publicado` when none was given.
      - **Sedes** — prefix strict venues with `SOLO`; use `PREFERIR` only for
        explicit favorites. Never label a strict venue scope as a preference.
      - **Personas** — the party size the CLI was invoked with (1–5),
        followed by the source phrase that justified it (for example
        `3 (somos tres)` or `1 (voy solo)`). If the user did not name
        a group, write `(? — preguntar)`. This line is independent
        from **Opciones**.
      - **Opciones** — the `--limit` value the CLI was invoked with
        (default 3), followed by the source phrase that justified it
        when it came from the user (for example `3 (top 3)` or
        `5 (dame 5 opciones)`). If no number of options was requested,
        write `3 (default)`. This line is independent from
        **Personas**; phrases like `top 3`, `dame 3 opciones` or
        `los 3 mejores` **never** set Personas.
      Omit unrequested preferences instead of printing `(no pedido)`. Convert
      `observed_at` to `America/Lima` for the compact clock, while retaining
      the exact timestamp only in the final freshness line. Report sold-out
      maps as `agotadas` and other failures as `fallas`; do not call an
      excluded sold-out function a partial map failure.

   b. **Opciones** — present up to three recommendations in order as open
      sections, never closed cards. Begin the first with `┌─ ⭐ 1 · HH:MM ·
      SEDE` and later ones with `┌─ 2 · ...`; end each with `└─ Elegir: N`.
      The next two lines are compact:

      ```text
      LIBRES libres · 2D/3D · DOB/SUB · PRIME/Regular
      ♟︎ G6, G7, G8 · Visión 94/100 Excelente · juntos · zona central media-trasera
      ```

      Translate enum/reason codes to natural Spanish. Never print raw values
      such as `central_middle_rear` or `contiguous_central_block`. Include
      every recommended seat label explicitly and in returned order; never
      use ranges or compressed labels.

      By default, show the complete stored `seat_preview` for each returned
      option (#1, #2, and #3): every row and column, without omissions.
      Preserve structurally empty rows when they represent room geometry, but
      do not add decorative padding outside the map. Put each complete map in
      a fenced `text` code block, with `PANTALLA` above it. Use the fixed
      palette above and no map borders, including no right-side border.
      `mapa N` may repeat that option's complete stored room as detail; it
      does not reveal hidden content.

      Insert this exact separator on its own line between option sections,
      after `└─ Elegir: N` and before the next `┌─`, with one blank line above
      and below it. Never place it after the final option:

      ```text
      ────────────────────────────────────────────────────────────
      ```

   c. **Legend and freshness** — end with `◻ disponible · ◼ ocupado · ♿
      accesibilidad · ♟︎ recomendado · dos espacios = pasillo/hueco` and the
      quoted `observed_at`, noting that availability can change.

   Keep the JSON `layout` ASCII exactly as returned. Do not infer,
   expose, or manufacture identifiers for the remaining seats.
5. State that availability was observed for this query and can change. Quote the `observed_at` value when describing how fresh the result is, and re-run the command if the user asks about anything more recent. The ranking first seeks a contiguous block. Only when no contiguous block exists may it accept the existing ranking's valid group split separated solely by an aisle; say explicitly that the group is split by that aisle. Never invent or present a dispersed split. Only a `diagnostics.map_failures.message` containing `Seat sold out` means that function is exhausted and excluded. A `Scattered` recommendation or one without a contiguous block can still have seats and is not sold out. Report all other `diagnostics.map_failures` as partial map failures; surface network, parse, and contract failures through the versioned JSON error envelope. Note that error envelopes intentionally do not include `observed_at` because they do not represent observed availability.

## Numbered follow-ups and revalidation

Keep a session-local result record for every numbered option: its rank,
recommendation `id`, `checkout_handoff.session_id`,
`checkout_handoff.session_fingerprint`, selected seat labels, and exact seat
selection URL. Interpret a follow-up such as "1", "la segunda" or
"continúa con 3" against that record.

A bare numbered follow-up (`1`, `2`, `3`, or equivalent) identifies and
selects that stored option, without creating a hold; it may also show its full
stored `seat_preview` as detail, without a new query. `mapa 1`, `mapa 2`, or
`mapa 3` only shows that stored full map and does not select an option or start
checkout. `reservar 1` or `continuar con 1` explicitly starts checkout:
the explicit reservation verb is itself the user's authorization. Invoke the
single `checkout --yes` command immediately, without asking for an additional
`y/N`, and let that command revalidate before creating a hold. The same applies
to `retener 1` and explicit requests to open checkout for a numbered option.
Requery separately only when the user asks for an updated recommendation
without checkout.

On checkout handoff, pass the prior recommendation `id`, never merely its
mutable rank, to the repository's `checkout` command. That command reruns the
same filtered recommendation query, finds the exact ID, validates the renewed
session fingerprint and URL, and refuses stale or mismatched selections. Do
not mix a seat label, URL, or session from different recommendations. A new
numbered choice starts a new selection record; it does not silently replace an
existing one.

## Checkout handoff

Natural-language recommendations are read-only by default. Do not open a
checkout merely because a user asks for movie times, seats, or a
recommendation.

An explicit request to **reserve**, **continue**, **hold**, or **open checkout**
for a selected or numbered option is the only handoff trigger. That request is
also the authorization to create the temporary guest hold; do not ask for a
second confirmation. In that case:

1. Do not manually rerun `recommend` and do not reuse its old seat state as
   proof of availability. Keep only the chosen recommendation's stable ID and
   original query. The `checkout` command performs the single authoritative
   revalidation immediately before interacting with Cineplanet and fails if
   that ID no longer has a valid `checkout_handoff`.
2. State that this is Playwright-assisted: Cineplanet encrypts `add-tickets`,
   and the resulting guest hold lives only in the dedicated persistent Chrome
   tab/session. The printed URL is informational and is not portable to a
   different browser profile.
3. **Never use Agent Browser, Browser MCP, Chrome skills, `node_repl`, manual
   DOM inspection, screenshots, coordinate clicks, or browser calls for this
   flow.** The repository owns the automation. After the explicit reservation
   request, invoke exactly one local `checkout` command with the same
   movie/city/date/venue/modality filters used for `recommend`, the stored
   recommendation ID, and `--yes`:

   ```bash
   cargo run --quiet --manifest-path "$(git -C . rev-parse --show-toplevel)/Cargo.toml" -- checkout \
     --movie-title "..." --city "..." --party-size 3 \
     --date YYYY-MM-DD --venue "..." --venue "..." \
     --recommendation-id "<stored recommendation id>" --yes
   ```

   The command uses the repo's Playwright runner to correlate the official
   seat-plan coordinates with Cineplanet's unlabeled visual controls, validates
   every requested seat as available, selects exactly that block, clicks
   Continue, chooses **Seguir como invitado**, and stops at `/entradas`. It
   leaves that persistent Chrome tab visible. If `playwright-core` is missing,
   tell the user to run `npm install` once in the repository and retry the same
   command; do not fall back to another browser tool.
4. Do not ask `y/N` or repeat a confirmation before invoking `checkout --yes`.
   Phrases such as `reservar 3`, `retener la opción 3`, `continuar con 3`, or
   `abrir checkout para la tercera` are already explicit approval. A bare
   number such as `3` still only selects the stored option and never creates a
   hold.
5. Trust success only when the command returns `status:
   ready_for_ticket_selection`, the exact selected labels, a nonzero group,
   and the `/entradas` URL. The runner reloads once if the page has no real
   fares. On success, state that the dedicated Chrome tab is already open and
   the user can choose entradas generales or beneficios there. Stop before
   choosing a ticket category, promotion, concessions or payment. Say the hold
   is roughly five minutes and expires with that browser session.

Never log in or choose a ticket category, promotion, concessions, payment
method, or submit payment. Do not proceed beyond `/entradas`.
