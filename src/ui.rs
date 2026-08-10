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
const GOLD: Color = Color::Rgb(255, 177, 47);

const WELCOME_BRAND: &[&str] = &[
    r"  ____  _              _   _       _    ",
    r" / ___|(_)_ __ ___    | | | | ___ | | __",
    r" \___ \| | '_ ` _ \   | |_| |/ _ \| |/ /",
    r"  ___) | | | | | | |  |  _  | (_) |   < ",
    r" |____/|_|_| |_| |_|  |_| |_|\___/|_|\_\",
];
const WELCOME_SUBTITLE: &str = "Cineplanet  ·  CineplanetCLI";
const WELCOME_TAGLINE: &str = "Encuentra tu mejor funcion";
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
        Span::styled(movie, Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_welcome(frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GOLD))
        .title(Line::from(vec![Span::styled(
            " Bienvenido ",
            Style::default()
                .fg(Color::Black)
                .bg(GOLD)
                .add_modifier(Modifier::BOLD),
        )]));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = WELCOME_BRAND.iter().map(|line| Line::from(*line)).collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_SUBTITLE,
        Style::default().add_modifier(Modifier::BOLD).fg(GOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_TAGLINE,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        WELCOME_INSTRUCTIONS,
        Style::default().fg(Color::DarkGray),
    )));

    let [content] = Layout::vertical([Constraint::Length(lines.len() as u16)])
        .flex(Flex::Center)
        .areas(inner);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), content);
}

fn render_venues(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let items: Vec<_> = app
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
    let items: Vec<_> = app
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
                Line::from(Span::styled(metadata, Style::default().fg(Color::DarkGray))),
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
    if app.recommendations().is_empty() {
        frame.render_widget(
            Paragraph::new("No encontramos bloques contiguos para este grupo.")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title(" Resultados ")),
            area,
        );
        return;
    }

    let items: Vec<_> = app
        .recommendations()
        .iter()
        .map(|recommendation| {
            let quality = match recommendation.quality {
                Quality::Excellent => "Excelente",
                Quality::Good => "Buena",
                Quality::Unfavorable => "Menos favorable",
            };
            let showtime = &recommendation.showtime;
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{quality:<16}"),
                        Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &showtime.venue_name,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(format!(
                    "{} · {} · {} · {}",
                    showtime.starts_at.format("%d/%m %H:%M"),
                    showtime.modality.projection_format,
                    showtime.modality.language,
                    showtime.modality.room_type,
                )),
                Line::from(Span::styled(
                    recommendation.reasons.join(" · "),
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.result_index()));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mejores funciones "),
        )
        .highlight_style(Style::default().fg(Color::White).bg(BLUE))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_seat_map(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(recommendation) = app.current_recommendation() else {
        return;
    };
    let map = &recommendation.showtime.seat_map;
    let recommended: BTreeSet<_> = recommendation
        .block
        .iter()
        .map(|seat| seat.id.as_str())
        .collect();

    let mut lines = vec![
        Line::from(Span::styled(
            "                         PANTALLA                         ",
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
            .unwrap_or("?");
        let mut spans = vec![Span::styled(
            format!("{row_label:>2}  "),
            Style::default().fg(Color::DarkGray),
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
                    SeatState::Occupied => ("■  ", Style::default().fg(Color::DarkGray)),
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
        Span::styled("■ ", Style::default().fg(Color::DarkGray)),
        Span::raw("ocupado"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Mapa de sala "),
            ),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let help = match app.screen() {
        Screen::Welcome => "Enter comenzar · Q / Ctrl-C salir",
        Screen::CitySetup => "↑↓ mover · Enter elegir ciudad · Q salir",
        Screen::VenueSetup => "↑↓ mover · Espacio marcar · Enter guardar · Q salir",
        Screen::Movies => "Escribe para filtrar · ↑↓ mover · Enter analizar · P sedes · Q salir",
        Screen::Loading => "Actualizando disponibilidad real…",
        Screen::Results => "↑↓ mover · Enter ver sala · Esc volver · P sedes · Q salir",
        Screen::SeatMap => "Esc volver · Q salir",
    };
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
