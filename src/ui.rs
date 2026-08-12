use std::collections::BTreeSet;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    app::{App, Screen},
    domain::{Quality, SeatState},
};

const BLUE: Color = Color::Rgb(16, 70, 135);
const PLANET_BLUE: Color = Color::Rgb(86, 137, 207);
const GOLD: Color = Color::Rgb(255, 177, 47);
const TEXT: Color = Color::Rgb(244, 247, 252);
const MUTED: Color = Color::Rgb(184, 196, 216);
const SUCCESS: Color = Color::Rgb(112, 214, 142);
const ALERT: Color = Color::Rgb(242, 112, 112);
const PRIME: Color = Color::Rgb(226, 170, 255);
const DUBBED: Color = Color::Rgb(105, 207, 255);
const SUBTITLED: Color = Color::Rgb(126, 220, 183);

const WELCOME_SUBTITLE: &str = "Cartelera y asientos de Cineplanet";
const WELCOME_TAGLINE: &str = "Encuentra tu mejor función";
const WELCOME_INSTRUCTIONS: &str = "[Enter]  comenzar       [Q]  /  [Ctrl-C]  salir";

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    render_header(frame, header, app);
    match app.screen() {
        Screen::Welcome => render_welcome(frame, body),
        Screen::CitySetup => render_city_setup(frame, body, app),
        Screen::VenueSetup => render_venues(frame, body, app),
        Screen::Movies => render_movies(frame, body, app),
        Screen::DateFilter => render_date_filter(frame, body, app),
        Screen::VenueFilter => render_venue_filter(frame, body, app),
        Screen::PartySize => render_party_size(frame, body, app),
        Screen::SearchSummary => render_search_summary(frame, body, app),
        Screen::Loading => render_loading(frame, body),
        Screen::Results => render_results(frame, body, app),
        Screen::SeatMap => render_seat_map(frame, body, app),
    }
    render_footer(frame, footer, app);
}

fn render_loading(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("Consultando mapas de asientos reales de Cineplanet…")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Actualizando disponibilidad "),
            ),
        area,
    );
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let movie = app
        .current_movie()
        .map(|movie| format!("  {}", movie.title))
        .unwrap_or_default();
    let title = Line::from(vec![
        Span::styled(
            " CineplanetCLI ",
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        if app.is_demo() {
            Span::styled(
                " MODO DEMO ",
                Style::default()
                    .fg(Color::Black)
                    .bg(GOLD)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " DATOS EN VIVO ",
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            )
        },
        Span::styled(movie, Style::default().fg(MUTED)),
    ]);
    frame.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_welcome(frame: &mut Frame<'_>, area: Rect) {
    let mut lines = welcome_brand();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_SUBTITLE,
        Style::default().add_modifier(Modifier::BOLD).fg(GOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_TAGLINE,
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_INSTRUCTIONS,
        Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
    )));

    let [content] = Layout::vertical([Constraint::Length(lines.len() as u16)])
        .flex(Flex::Center)
        .areas(area);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), content);
}

fn welcome_brand() -> Vec<Line<'static>> {
    let planet = Style::default().fg(PLANET_BLUE);

    vec![
        Line::from(Span::styled(
            "  ░██████  ░██████░███    ░██ ░██████████ ░█████████  ░██            ░███    ░███    ░██ ░██████████ ░██████████",
            planet,
        )),
        Line::from(Span::styled(
            " ░██   ░██   ░██  ░████   ░██ ░██         ░██     ░██ ░██           ░██░██   ░████   ░██ ░██             ░██    ",
            planet,
        )),
        Line::from(Span::styled(
            "░██          ░██  ░██░██  ░██ ░██         ░██     ░██ ░██          ░██  ░██  ░██░██  ░██ ░██             ░██    ",
            planet,
        )),
        Line::from(Span::styled(
            "░██          ░██  ░██ ░██ ░██ ░█████████  ░█████████  ░██         ░█████████ ░██ ░██ ░██ ░█████████      ░██    ",
            planet,
        )),
        Line::from(Span::styled(
            "░██          ░██  ░██  ░██░██ ░██         ░██         ░██         ░██    ░██ ░██  ░██░██ ░██             ░██    ",
            planet,
        )),
        Line::from(Span::styled(
            " ░██   ░██   ░██  ░██   ░████ ░██         ░██         ░██         ░██    ░██ ░██   ░████ ░██             ░██    ",
            planet,
        )),
        Line::from(Span::styled(
            "  ░██████  ░██████░██    ░███ ░██████████ ░██         ░██████████ ░██    ░██ ░██    ░███ ░██████████     ░██    ",
            planet,
        )),
    ]
}

fn render_venues(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items: Vec<_> = app
        .visible_venues()
        .into_iter()
        .map(|venue| {
            let checked = app.preferences().favorite_venue_ids.contains(&venue.id);
            ListItem::new(format!(
                "{} {}",
                if checked { "[x]" } else { "[ ]" },
                venue.name
            ))
        })
        .collect();
    let continue_style = if app.venue_setup_on_continue() {
        Style::default()
            .fg(Color::White)
            .bg(BLUE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "Continuar",
        continue_style,
    ))));
    let mut state = ListState::default().with_selected(Some(app.venue_index()));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Sedes favoritas "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_city_setup(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items: Vec<_> = app
        .available_cities()
        .into_iter()
        .map(|city| {
            let marker = if app.preferences().city.as_deref() == Some(city) {
                "(actual) "
            } else {
                ""
            };
            ListItem::new(format!("{marker}{city}"))
        })
        .collect();
    let continue_style = if app.city_setup_on_continue() {
        Style::default()
            .fg(Color::White)
            .bg(BLUE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "Continuar",
        continue_style,
    ))));
    let mut state = if items.is_empty() {
        ListState::default()
    } else {
        ListState::default().with_selected(Some(app.city_index()))
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Elige tu ciudad "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_movies(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let [search, list_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).areas(area);
    frame.render_widget(
        Paragraph::new(format!("{}▌", app.query())).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Buscar película "),
        ),
        search,
    );

    let movies = app.visible_movies();
    let items: Vec<_> = movies
        .iter()
        .map(|movie| {
            let metadata = [
                movie.genre.as_deref(),
                movie.rating.as_deref(),
                movie
                    .duration_minutes
                    .map(|minutes| format!("{minutes} min"))
                    .as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            ListItem::new(vec![
                Line::from(Span::styled(
                    &movie.title,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(metadata, Style::default().fg(MUTED))),
            ])
        })
        .collect();
    let mut state = if items.is_empty() {
        ListState::default()
    } else {
        ListState::default().with_selected(Some(app.movie_index()))
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Cartelera "))
        .highlight_style(Style::default().fg(Color::White).bg(BLUE))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn render_results(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = if app.filters_active() {
        format!(
            " Funciones disponibles ({} fechas, {} sedes) ",
            app.selected_filter_dates().len(),
            app.selected_filter_venues().len()
        )
    } else {
        " Funciones disponibles ".to_string()
    };

    let mut items = vec![ListItem::new(vec![
        Line::from(Span::styled(
            "Modificar búsqueda",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Cambiar fechas o sedes antes de volver a consultar.",
            Style::default().fg(MUTED),
        )),
    ])];
    items.extend(app.result_showtimes().iter().map(|showtime| {
        let available = app.available_seat_count(showtime);
        let (availability, availability_style) = if showtime.seat_map.seats.is_empty() {
            (
                "SIN CONFIRMAR".to_string(),
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            )
        } else if available == 0 {
            (
                "0 asientos".to_string(),
                Style::default().fg(ALERT).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                format!("{available} asientos"),
                Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
            )
        };
        let room_style = if showtime.modality.room_type.eq_ignore_ascii_case("prime") {
            Style::default().fg(PRIME).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD)
        };
        let language_style = if showtime.modality.language.eq_ignore_ascii_case("doblada") {
            Style::default().fg(DUBBED).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SUBTITLED).add_modifier(Modifier::BOLD)
        };
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("{}  ", showtime.starts_at.format("%H:%M")),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{availability}  "), availability_style),
                Span::styled(
                    &showtime.venue_name,
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(
                [
                    Span::styled(
                        format!("[{}] ", showtime.modality.room_type.to_uppercase()),
                        room_style,
                    ),
                    Span::styled(
                        format!("[{}] ", showtime.modality.language.to_uppercase()),
                        language_style,
                    ),
                    Span::styled(
                        format!("[{}] ", showtime.modality.projection_format.to_uppercase()),
                        Style::default()
                            .fg(PLANET_BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]
                .into_iter()
                .chain(result_status_tags(app, showtime))
                .collect::<Vec<_>>(),
            ),
        ])
    }));
    if app.result_showtimes().is_empty() {
        items.push(ListItem::new("No hay funciones para esta combinación."));
    }
    let mut state = ListState::default().with_selected(Some(app.result_index()));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(BLUE).add_modifier(Modifier::BOLD))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn result_status_tags(app: &App, showtime: &crate::domain::Showtime) -> Vec<Span<'static>> {
    let bold = Modifier::BOLD;
    if showtime.seat_map.seats.is_empty() {
        return vec![Span::styled(
            "[MAPA NO DISPONIBLE]",
            Style::default().fg(MUTED).add_modifier(bold),
        )];
    }

    let party_size = app.preferences().party_size;
    let Some(analysis) = app.analyze_showtime(showtime) else {
        return vec![Span::styled(
            format!("[SIN {party_size} JUNTOS]"),
            Style::default().fg(ALERT).add_modifier(bold),
        )];
    };
    if analysis.block.len() != party_size {
        return vec![Span::styled(
            format!("[SIN {party_size} JUNTOS]"),
            Style::default().fg(ALERT).add_modifier(bold),
        )];
    }

    let group_tag = if party_size == 1 {
        "[ASIENTO DISPONIBLE]".to_string()
    } else {
        format!("[{party_size} JUNTOS]")
    };
    let (zone_tag, zone_style) = match analysis.quality {
        Quality::Excellent => (
            "[ZONA IDEAL]",
            Style::default().fg(SUCCESS).add_modifier(bold),
        ),
        Quality::Good => ("[ZONA BUENA]", Style::default().fg(GOLD).add_modifier(bold)),
        Quality::Unfavorable => (
            "[ZONA POCO FAVORABLE]",
            Style::default().fg(ALERT).add_modifier(bold),
        ),
    };
    vec![
        Span::styled(
            format!("{group_tag} "),
            Style::default().fg(TEXT).add_modifier(bold),
        ),
        Span::styled(zone_tag, zone_style),
    ]
}

fn render_date_filter(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items = Vec::new();
    for date in app.filter_dates() {
        let checked = app.selected_filter_dates().contains(date);
        let label = if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            parsed.format("%d/%m/%Y").to_string()
        } else {
            date.clone()
        };
        items.push(ListItem::new(format!(
            "{} {}",
            if checked { "[x]" } else { "[ ]" },
            label
        )));
    }

    let continue_style = if app.date_filter_on_continue() {
        if app.has_selected_dates() {
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        }
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "Continuar",
        continue_style,
    ))));
    let mut state = ListState::default().with_selected(Some(app.date_filter_index()));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ¿Qué fechas te funcionan? "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_venue_filter(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items = Vec::new();
    for venue_id in app.filter_venues() {
        let checked = app.selected_filter_venues().contains(venue_id);
        let venue_name = app
            .catalog()
            .venues
            .iter()
            .find(|v| v.id == *venue_id)
            .map(|v| v.name.as_str())
            .unwrap_or(venue_id);
        items.push(ListItem::new(format!(
            "{} {}",
            if checked { "[x]" } else { "[ ]" },
            venue_name
        )));
    }

    let continue_style = if app.venue_filter_on_continue() {
        if app.has_selected_venues() {
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        }
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "Continuar",
        continue_style,
    ))));

    let mut state = ListState::default().with_selected(Some(app.venue_filter_index()));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ¿En qué sedes? "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_party_size(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut items: Vec<_> = (1..=5)
        .map(|size| {
            let marker = if app.preferences().party_size == size {
                "(actual) "
            } else {
                ""
            };
            ListItem::new(format!(
                "{marker}{size} {}",
                if size == 1 { "persona" } else { "personas" }
            ))
        })
        .collect();
    let continue_style = if app.party_size_on_continue() {
        Style::default()
            .fg(Color::White)
            .bg(BLUE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "Continuar",
        continue_style,
    ))));
    let mut state = ListState::default().with_selected(Some(app.party_size_index()));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ¿Cuántas personas irán? "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_search_summary(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let dates = app
        .filter_dates()
        .iter()
        .filter(|date| app.selected_filter_dates().contains(*date))
        .map(|date| date.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let venues = app
        .filter_venues()
        .iter()
        .filter(|id| app.selected_filter_venues().contains(*id))
        .filter_map(|id| app.catalog().venues.iter().find(|venue| venue.id == *id))
        .map(|venue| venue.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let movie = app
        .current_movie()
        .map(|movie| movie.title.as_str())
        .unwrap_or("-");
    let lines = vec![
        Line::from(Span::styled(
            "Revisa tu búsqueda",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Película: {movie}")),
        Line::from(format!("Fechas: {dates}")),
        Line::from(format!("Sedes: {venues}")),
        Line::from(format!("Grupo: {} personas", app.preferences().party_size)),
        Line::from(""),
        Line::from(Span::styled(
            "[ Buscar funciones ]",
            Style::default()
                .fg(Color::White)
                .bg(BLUE)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Listo para consultar "),
            ),
        area,
    );
}

fn render_seat_map(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(showtime) = app.current_result_showtime() else {
        return;
    };
    let map = &showtime.seat_map;
    if map.seats.is_empty() {
        frame.render_widget(
            Paragraph::new("Cineplanet no pudo confirmar el mapa de esta función.")
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Mapa de sala "),
                ),
            area,
        );
        return;
    }
    let recommended: BTreeSet<_> = app
        .current_recommendation()
        .map(|recommendation| {
            recommendation
                .block
                .iter()
                .map(|seat| seat.id.clone())
                .collect()
        })
        .unwrap_or_default();

    let useful_map_width = usize::from(map.columns).saturating_mul(3).saturating_add(4);
    let screen_width = useful_map_width.min(usize::from(area.width.saturating_sub(2)));
    let screen = if screen_width >= " PANTALLA ".len() {
        format!("{:^screen_width$}", " PANTALLA ")
    } else {
        " ".repeat(screen_width)
    };
    let mut lines = vec![
        Line::from(Span::styled(
            screen,
            Style::default().fg(Color::White).bg(BLUE),
        )),
        Line::from(""),
    ];
    for y in 0..map.rows {
        let row_label = map
            .seats
            .iter()
            .find(|seat| seat.y == y)
            .map(|seat| seat.row.as_str())
            .unwrap_or("");
        let mut spans = vec![Span::styled(
            format!("{row_label:>2}  "),
            Style::default().fg(MUTED),
        )];
        for x in 0..map.columns {
            let Some(seat) = map.seats.iter().find(|seat| seat.x == x && seat.y == y) else {
                spans.push(Span::raw("   "));
                continue;
            };
            let (symbol, style) = if recommended.contains(seat.id.as_str()) {
                ("██ ", Style::default().fg(Color::Black).bg(GOLD))
            } else {
                match seat.state {
                    SeatState::Available => ("□  ", Style::default().fg(Color::Blue)),
                    SeatState::Occupied => ("■  ", Style::default().fg(MUTED)),
                    SeatState::Accessible => ("◇  ", Style::default().fg(Color::Cyan)),
                }
            };
            spans.push(Span::styled(symbol, style));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("██ ", Style::default().fg(Color::Black).bg(GOLD)),
        Span::raw(" recomendado   "),
        Span::styled("□ ", Style::default().fg(Color::Blue)),
        Span::raw("disponible   "),
        Span::styled("■ ", Style::default().fg(MUTED)),
        Span::raw("ocupado   "),
        Span::styled("◇ ", Style::default().fg(Color::Cyan)),
        Span::raw("accesibilidad"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Mapa real de Cineplanet · {} disponibles ",
                app.available_seat_count(showtime)
            ))),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = match app.screen() {
        Screen::Welcome => "Enter comenzar · Q / Ctrl-C salir",
        Screen::CitySetup => "↑↓ mover · Enter elegir ciudad · Q salir",
        Screen::VenueSetup => "↑↓ mover · Espacio marcar · Enter guardar · Q salir",
        Screen::Movies => {
            "Escribe para filtrar · ↑↓ mover · Enter elegir película · Esc sedes · Q salir"
        }
        Screen::DateFilter => {
            "↑↓ mover · Espacio/Enter marcar · baja a Continuar · Esc volver · Q salir"
        }
        Screen::VenueFilter => {
            "↑↓ mover · Espacio/Enter marcar · baja a Continuar · Esc volver · Q salir"
        }
        Screen::PartySize => "↑↓ mover · Enter elegir · baja a Continuar · Esc volver · Q salir",
        Screen::SearchSummary => "Enter buscar funciones · Esc volver · Q salir",
        Screen::Loading => "Actualizando disponibilidad real…",
        Screen::Results => "↑↓ mover · Enter ver o modificar · Esc volver · Q salir",
        Screen::SeatMap => "Esc volver · Q salir",
    };
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        app::{Action, App, Effect, Screen},
        demo,
        domain::{Catalog, Preferences, Seat, SeatMap, SeatState},
    };

    use super::render;

    fn app_with_results() -> App {
        app_with_results_from_catalog(
            demo::catalog(),
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                ..Preferences::default()
            },
        )
    }

    fn app_with_results_from_catalog(catalog: Catalog, preferences: Preferences) -> App {
        let mut app = App::new(catalog, preferences);
        app.apply(Action::Confirm).unwrap();
        app.apply(Action::Confirm).unwrap();

        for _ in 0..app.filter_dates().len() {
            app.apply(Action::Toggle).unwrap();
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        for _ in 0..app.filter_venues().len() {
            app.apply(Action::Toggle).unwrap();
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        while !app.party_size_on_continue() {
            app.apply(Action::Down).unwrap();
        }
        assert_eq!(app.apply(Action::Confirm).unwrap(), Effect::SavePreferences);
        assert!(matches!(
            app.apply(Action::Confirm).unwrap(),
            Effect::FetchSeatMaps(_)
        ));
        let showtimes = app.showtimes_to_hydrate();
        app.finish_loading_showtimes(showtimes);
        assert_eq!(app.screen(), Screen::Results);
        app
    }

    fn rendered_lines(app: &App) -> Vec<String> {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn results_render_each_showtime_as_two_labeled_lines() {
        let app = app_with_results();
        let lines = rendered_lines(&app);
        let first = &app.result_showtimes()[0];
        let second = &app.result_showtimes()[1];

        assert!(lines[6].contains(&first.starts_at.format("%H:%M").to_string()));
        assert!(lines[6].contains("asientos"));
        assert!(lines[6].contains(&first.venue_name));
        assert!(lines[7].contains("REGULAR") || lines[7].contains("PRIME"));
        assert!(lines[7].contains("DOBLADA") || lines[7].contains("SUBTITULADA"));
        assert!(lines[7].contains("2D") || lines[7].contains("3D"));
        assert!(
            lines[7].contains("JUNTOS")
                || lines[7].contains("ASIENTO")
                || lines[7].contains("MAPA")
        );
        assert!(lines[8].contains(&second.starts_at.format("%H:%M").to_string()));
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("asientos disponibles"))
        );
    }

    #[test]
    fn results_render_group_and_zone_tags_for_each_showtime_analysis() {
        let mut catalog = demo::catalog();
        let mut showtimes: Vec<_> = catalog
            .showtimes
            .iter()
            .filter(|showtime| showtime.movie_id == "spider-man")
            .cloned()
            .collect();
        showtimes[0].seat_map = seat_map(&[(6, 4), (6, 5)]);
        showtimes[1].seat_map = seat_map(&[(3, 4), (3, 5)]);
        showtimes[2].seat_map = seat_map(&[(9, 0), (9, 1)]);
        showtimes[3].seat_map = seat_map(&[(6, 4), (6, 6)]);
        let mut unavailable = showtimes[0].clone();
        unavailable.id = "map-unavailable".into();
        unavailable.starts_at += Duration::minutes(1);
        unavailable.seat_map = SeatMap {
            rows: 0,
            columns: 0,
            seats: Vec::new(),
        };
        showtimes.push(unavailable);
        catalog.showtimes = showtimes;

        let app = app_with_results_from_catalog(
            catalog,
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                party_size: 2,
                ..Preferences::default()
            },
        );
        let lines = rendered_lines(&app);

        assert!(lines.iter().any(|line| line.contains("[2 JUNTOS]")));
        assert!(lines.iter().any(|line| line.contains("[ZONA IDEAL]")));
        assert!(lines.iter().any(|line| line.contains("[ZONA BUENA]")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("[ZONA POCO FAVORABLE]"))
        );
        assert!(lines.iter().any(|line| line.contains("[SIN 2 JUNTOS]")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("[MAPA NO DISPONIBLE]"))
        );
    }

    #[test]
    fn results_render_a_single_available_seat_tag_for_one_person() {
        let mut catalog = demo::catalog();
        catalog
            .showtimes
            .retain(|showtime| showtime.movie_id == "spider-man");
        catalog.showtimes.truncate(1);
        catalog.showtimes[0].seat_map = seat_map(&[(6, 4)]);
        let app = app_with_results_from_catalog(
            catalog,
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                party_size: 1,
                ..Preferences::default()
            },
        );

        assert!(
            rendered_lines(&app)
                .iter()
                .any(|line| line.contains("[ASIENTO DISPONIBLE]"))
        );
    }

    #[test]
    fn results_normalize_a_persisted_zero_person_party_before_rendering() {
        let app = app_with_results_from_catalog(
            demo::catalog(),
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                party_size: 0,
                ..Preferences::default()
            },
        );

        assert_eq!(app.preferences().party_size, 1);
        assert!(
            rendered_lines(&app)
                .iter()
                .all(|line| !line.contains("[SIN 0 JUNTOS]"))
        );
    }

    #[test]
    fn current_map_analysis_keeps_a_block_for_an_unfavorable_result() {
        let mut catalog = demo::catalog();
        let mut showtimes: Vec<_> = catalog
            .showtimes
            .iter()
            .filter(|showtime| showtime.movie_id == "spider-man")
            .cloned()
            .collect();
        showtimes.truncate(2);
        showtimes[0].id = "apt".into();
        showtimes[0].seat_map = seat_map(&[(6, 4), (6, 5)]);
        showtimes[1].id = "unfavorable".into();
        showtimes[1].seat_map = seat_map(&[(9, 0), (9, 1)]);
        catalog.showtimes = showtimes;

        let mut app = app_with_results_from_catalog(
            catalog,
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                party_size: 2,
                ..Preferences::default()
            },
        );
        assert_eq!(app.recommendations().len(), 1);
        assert_eq!(app.recommendations()[0].showtime.id, "apt");

        let unfavorable_index = app
            .result_showtimes()
            .iter()
            .position(|showtime| showtime.id == "unfavorable")
            .unwrap();
        for _ in 0..=unfavorable_index {
            app.apply(Action::Down).unwrap();
        }
        app.apply(Action::Confirm).unwrap();
        assert_eq!(app.screen(), Screen::SeatMap);

        let analysis = app.current_recommendation().unwrap();
        assert_eq!(analysis.quality, crate::domain::Quality::Unfavorable);
        assert_eq!(analysis.block.len(), 2);
        assert_eq!(analysis.showtime.id, "unfavorable");
    }

    #[test]
    fn results_show_no_group_tag_for_a_nonempty_map_with_zero_available_seats() {
        let mut catalog = demo::catalog();
        catalog
            .showtimes
            .retain(|showtime| showtime.movie_id == "spider-man");
        catalog.showtimes.truncate(1);
        catalog.showtimes[0].seat_map = seat_map(&[]);
        let app = app_with_results_from_catalog(
            catalog,
            Preferences {
                onboarding_complete: true,
                city: Some("Lima".into()),
                party_size: 2,
                ..Preferences::default()
            },
        );

        let lines = rendered_lines(&app);
        assert!(lines.iter().any(|line| line.contains("[SIN 2 JUNTOS]")));
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("[MAPA NO DISPONIBLE]"))
        );
    }

    fn seat_map(available: &[(u16, u16)]) -> SeatMap {
        let seats = (0..10)
            .flat_map(|y| {
                (0..10).map(move |x| Seat {
                    id: format!("{}{}", (b'A' + y as u8) as char, x + 1),
                    row: ((b'A' + y as u8) as char).to_string(),
                    number: x + 1,
                    x,
                    y,
                    state: if available.contains(&(y, x)) {
                        SeatState::Available
                    } else {
                        SeatState::Occupied
                    },
                })
            })
            .collect();
        SeatMap {
            rows: 10,
            columns: 10,
            seats,
        }
    }
}
