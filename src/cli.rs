use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cineplanet-cli",
    about = "Cartelera y recomendaciones de Cineplanet"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inicia la aplicación interactiva de terminal.
    Tui,
    /// Produce recomendaciones como JSON, sin interfaz interactiva.
    Recommend(Box<RecommendArgs>),
    /// Revalida una recomendación y abre /entradas como invitado.
    Checkout(Box<CheckoutArgs>),
}

#[derive(Debug, Clone, Args)]
pub struct CheckoutArgs {
    #[command(flatten)]
    pub recommend: RecommendArgs,
    /// ID exacto devuelto por `recommend`; no se acepta un rank mutable.
    #[arg(long)]
    pub recommendation_id: String,
    /// Confirma que se puede crear la retención temporal de las butacas.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
#[command(group(ArgGroup::new("movie").required(true).args(["movie_id", "movie_title"])))]
pub struct RecommendArgs {
    #[arg(long, group = "movie")]
    pub movie_id: Option<String>,
    #[arg(long, group = "movie")]
    pub movie_title: Option<String>,
    #[arg(long)]
    pub city: String,
    #[arg(long, value_parser = parse_party_size)]
    pub party_size: usize,
    #[arg(long = "date")]
    pub dates: Vec<String>,
    #[arg(long = "venue")]
    pub venues: Vec<String>,
    #[arg(long = "language")]
    pub languages: Vec<String>,
    #[arg(long = "format")]
    pub formats: Vec<String>,
    #[arg(long = "room-type")]
    pub room_types: Vec<String>,
    #[arg(long = "favorite-venue")]
    pub favorite_venues: Vec<String>,
    #[arg(long, default_value_t = 3, value_parser = parse_positive)]
    pub limit: usize,
}

fn parse_party_size(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| "debe ser un entero entre 1 y 5".to_string())?;
    (1..=5)
        .contains(&value)
        .then_some(value)
        .ok_or_else(|| "debe estar entre 1 y 5".to_string())
}

fn parse_positive(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| "debe ser un entero positivo".to_string())?;
    (value > 0)
        .then_some(value)
        .ok_or_else(|| "debe ser mayor que cero".to_string())
}
