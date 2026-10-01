# CineplanetCLI

![Rust](https://img.shields.io/badge/Rust-CE422B?style=flat&logo=rust&logoColor=white)
![Ratatui](https://img.shields.io/badge/Ratatui-FFC131?style=flat&logoColor=black)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=flat&logoColor=white)

Encuentra funciones de Cineplanet y buenos asientos desde la terminal. Compara
sedes, horarios y bloques para grupos de 1 a 5 personas con disponibilidad en vivo.

- **TUI:** explora la cartelera y los mapas de asientos con el teclado.
- **CLI:** obtén recomendaciones en JSON para scripts y agentes como Codex o Claude Code.

## Demos

| TUI · Interfaz interactiva | CLI · Consultas y recomendaciones |
| --- | --- |
| [Ver demo de la TUI](https://lnkd.in/p/gfk-ZKPa) | [Ver demo del CLI](https://lnkd.in/p/grEzbweK) |

[![Vista de CineplanetCLI](https://i.postimg.cc/nLpQTNqN/image.png)](https://postimg.cc/w14vjffk)

## Instalación

Necesitas Rust estable. En macOS, instala también las herramientas de línea de
comandos de Xcode (`xcode-select --install`). macOS es la plataforma verificada.

```bash
git clone https://github.com/asther0/cineplanet-cli.git
cd cineplanet-cli
cargo install --path .
```

Para usar el checkout, necesitas además **Node.js 20+**, **Google Chrome** y
las dependencias del repositorio:

```bash
npm install
```

Esto instala Playwright Core y utiliza tu Chrome existente.

## TUI

```bash
cineplanet-cli tui
```

También se abre con `cineplanet-cli` sin argumentos. El flujo es:

```text
Ciudad → película → fechas → sedes → grupo → funciones → mapa de asientos
```

Usa las **flechas** para moverte, **Enter** para continuar, **Espacio** para
selección múltiple y **Esc** para volver. Escribe para filtrar la lista,
borra con **Backspace** y sal con **Q**.

## CLI

Reemplaza el título por una película en cartelera:

```bash
cineplanet-cli recommend \
  --movie-title "La Odisea" --city Lima \
  --party-size 2 --venue "La Molina" --venue "Salaverry" --limit 3
```

`--party-size` indica cuántas personas van (1–5); `--limit`, cuántas opciones
mostrar. La película (`--movie-title` o `--movie-id`), la ciudad y el tamaño
del grupo son obligatorios.

Puedes añadir filtros con `--date YYYY-MM-DD`, `--language Subtitulada`,
`--format 2D` y `--room-type Regular`. Repite un filtro para incluir varios
valores. Las fechas corresponden a America/Lima. Usa `--venue` para restringir
sedes o `--favorite-venue` para priorizarlas.

El comando devuelve **JSON v1** por stdout con recomendaciones ordenadas,
asientos sugeridos, puntuación de visión, mapas y momento de consulta
(`observed_at`). Los errores salen como JSON por stderr. No requiere un LLM
ni abre el navegador.

Para ejecutarlo sin instalar, usa `cargo run --quiet -- recommend ...`.
Consulta todas las opciones con `cineplanet-cli recommend --help`.

### Uso con agentes

La [skill local cineplanet-recommend](.agents/skills/cineplanet-recommend/SKILL.md)
permite convertir peticiones como «busca las tres mejores funciones en
Salaverry para dos personas» en consultas del CLI. Otros agentes pueden
invocar `recommend` y consumir su JSON directamente.

### Continuar con una reserva

Reutiliza los filtros de la consulta y el `id` de la recomendación elegida:

```bash
cineplanet-cli checkout \
  --movie-title "La Odisea" --city Lima \
  --party-size 2 --venue "La Molina" --venue "Salaverry" --limit 3 \
  --recommendation-id "<id-devuelto-por-recommend>" --yes
```

El checkout revalida los asientos, abre Chrome, continúa como invitado y deja
la sesión en `/entradas` con una retención temporal. Tú eliges la tarifa,
las promociones y completas el pago. Los agentes deben ejecutarlo solo
cuando el usuario lo solicite explícitamente.

La TUI y `recommend` son de solo lectura. La disponibilidad puede cambiar
entre la consulta y la compra.

## Proyecto derivado

[cineplanet-api](https://github.com/gersonsebastianx/cineplanet-api), desarrollado
por [gersonsebastianx](https://github.com/gersonsebastianx) a partir de CineplanetCLI,
ofrece una interfaz web conversacional con IA.
[Prueba el demo web](https://cineplanet-api.vercel.app).
