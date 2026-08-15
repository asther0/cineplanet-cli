# CineplanetCLI

![Rust](https://img.shields.io/badge/Rust-CE422B?style=flat&logo=rust&logoColor=white)
![Ratatui](https://img.shields.io/badge/Ratatui-FFC131?style=flat&logoColor=black)
![Crossterm](https://img.shields.io/badge/Crossterm-2D7DD2?style=flat&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=flat&logoColor=white)
![Reqwest](https://img.shields.io/badge/Reqwest-6E2EB1?style=flat&logoColor=white)
![Playwright](https://img.shields.io/badge/Playwright-2EAD33?style=flat&logo=playwright&logoColor=white)
![Node.js](https://img.shields.io/badge/Node.js-5FA04E?style=flat&logo=nodedotjs&logoColor=white)

[![image.png](https://i.postimg.cc/nLpQTNqN/image.png)](https://postimg.cc/w14vjffk)

Encuentra funciones de Cineplanet con buenos asientos, sin revisar sede por sede.

La interfaz principal es un CLI determinista para agentes: Codex, Claude Code,
Cursor, OpenCode o cualquier otro harness puede ejecutar `recommend`, leer un
JSON estable y decidir qué opción mostrarte. También incluye una TUI para uso
humano directo en la terminal.

```text
Agente / harness ── recommend ──► JSON v1 ──► comparación y recomendación
                                  │
                                  └── checkout --yes ──► Chrome visible /entradas

Persona ─────────── tui ─────────► selección interactiva con flechas
```

```text
┌ CineplanetCLI ───────────────────────────────────────────┐
│ Película: Spider-Man                                    │
│ Grupo: 2 personas                                       │
├ Funciones disponibles ──────────────────────────────────┤
│ > 20:30  55 asientos  CP La Molina                       │
│   21:00  18 asientos  CP Salaverry                       │
└ ↑↓ elegir  Enter ver o modificar  Esc volver  Q salir ─┘
```

## Valor y capacidades actuales

CineplanetCLI resuelve la parte tediosa de buscar entradas: compara muchas
funciones y sedes en una sola consulta y te dice dónde hay asientos que sí
valen la pena, no solo cuántos quedan.

- Consulta cartelera y mapas públicos en vivo, sin login.
- Filtra por ciudad, película, fechas publicadas, sedes, idioma, formato y tipo de sala.
- Compara hasta las mejores opciones y muestra hora, sede, modalidad y asientos disponibles.
- Busca bloques contiguos para grupos de 1 a 5; si solo existe una separación por pasillo, la explica.
- Puntúa la visión según centro horizontal y zona media/media-trasera, evitando extremos, primeras filas y accesibilidad.
- Devuelve mapas completos y legibles, con cada asiento recomendado identificado.
- Puede revalidar una opción, seleccionar las butacas en Chrome y dejarte en `/entradas` como invitado; tú eliges tarifa, promociones y pago.

## Proyecto derivado

Este proyecto también dio lugar a una interfaz web y una API reutilizable para
consultar Cineplanet sin instalar el CLI:

| Proyecto | Repositorio | Demo |
| --- | --- | --- |
| **cineplanet-api**, de [gersonsebastianx](https://github.com/gersonsebastianx): experiencia web conversacional con IA para consultar funciones y encontrar entradas de Cineplanet. | [GitHub](https://github.com/gersonsebastianx/cineplanet-api) | [cineplanet-api.vercel.app](https://cineplanet-api.vercel.app) |

## Flujo

```text
Ciudad → película → fecha(s) → sede(s) → grupo (1-5) → funciones → mapa
```

La TUI permite escribir para filtrar ciudades, películas, fechas, sedes y funciones al instante. También funciona con flechas, Enter, Escape y selectores múltiples; no requiere memorizar comandos.

## Uso principal: CLI para Codex y otros harnesses

`recommend` es la interfaz recomendada para automatización. No abre una
interfaz ni depende de un LLM: escribe exactamente un documento JSON `v1` en
stdout. Los errores también tienen un sobre JSON versionado y salen por stderr,
por lo que un harness puede separar datos y errores de forma determinista.

Desde el repositorio:

```bash
cargo run --quiet -- recommend \
  --movie-title "La Odisea" --city "Lima" --party-size 2 \
  --date 2026-08-15 \
  --venue "CP La Molina" --venue "Salaverry" \
  --language Subtitulada --format 2D --room-type Regular \
  --limit 3 > cineplanet-result.json
```

Con una instalación global es equivalente:

```bash
cineplanet-cli recommend --movie-title "La Odisea" --city Lima \
  --party-size 2 --date 2026-08-15 --venue "La Molina" --limit 3
```

El contrato está pensado para que otro programa lo consuma directamente:

```bash
cineplanet-cli recommend \
  --movie-title "La Odisea" --city Lima --party-size 3 \
  --date 2026-08-15 --venue "San Miguel" --venue "Salaverry" \
  --limit 3 \
  | jq '.recommendations[] | {rank, venue, starts_at, seats: .checkout_handoff.selected_seat_labels, score: .viewing.score}'
```

Un harness puede seguir este ciclo:

1. Convertir la petición humana en `--movie-title`/`--movie-id`, `--city`,
   `--party-size`, fechas, sedes y `--limit`.
2. Ejecutar `recommend` una sola vez y parsear el JSON `v1`.
3. Presentar hora, sede, modalidad, asientos explícitos, visión y el mapa
   `seat_preview` de cada opción.
4. Conservar el `recommendations[].id` de la opción que el usuario elija.
5. Solo si el usuario pide reservar, ejecutar `checkout` con ese ID y `--yes`.

En Codex, la skill repo-scoped `$cineplanet-recommend` se descubre desde
`.agents/skills/cineplanet-recommend` y traduce peticiones como «busca las tres
mejores funciones de La Odisea esta noche en San Miguel para tres personas» a
este flujo. En Claude Code, OpenCode, Cursor u otro harness, basta con darle el
comando anterior como herramienta local y pedirle que trate stdout como JSON;
no necesita conocer la TUI ni usar un navegador para consultar disponibilidad.

### Retención opcional desde un agente

`recommend` es de solo lectura. El checkout es una acción separada y solo debe
ejecutarse después de una instrucción explícita como `reservar opción 2` o
`continuar con 2`:

```bash
cineplanet-cli checkout \
  --movie-title "La Odisea" --city Lima --party-size 3 \
  --date 2026-08-15 --venue "San Miguel" --venue "Salaverry" \
  --recommendation-id "<id-devuelto-por-recommend>" --yes
```

El comando vuelve a validar el ID y los asientos, abre Chrome visible, elige
**Seguir como invitado** y deja la sesión en `/entradas`. El usuario escoge la
tarifa, promociones y pago. La retención dura aproximadamente cinco minutos y
vive en esa sesión de Chrome. Un número solo (`2`) selecciona una opción pero
no crea ninguna retención.

El selector de película es exactamente uno de `--movie-id` o `--movie-title`; ciudad y grupo (`1` a `5`) son obligatorios. Repite `--date`, `--venue`, `--language`, `--format`, `--room-type` y `--favorite-venue` para varios valores. Las fechas usan `YYYY-MM-DD` en America/Lima. La respuesta `v1` contiene `observed_at` (RFC 3339 en UTC, momento en que la CLI observó la disponibilidad), `query`, recomendaciones ordenadas con `venue`, `starts_at`, `modality`, `viewing` (score, calidad, zona y razones), `selected_block`, `seat_preview` y `diagnostics` (`candidate_count`, `hydrated_count`, `map_failures`). `seat_preview` es compacto y determinista: `PANTALLA` queda arriba, las filas se ordenan como el mapa observado y su `layout` usa `.` disponible, `#` ocupado, `A` accesibilidad, `*` asiento recomendado y espacios para pasillos o huecos. No expone IDs remotos de los demás asientos. Cuando la fuente live entrega los IDs oficiales, cada recomendación añade `checkout_handoff` con slug, sede, sesión, huella `session_fingerprint`, URL oficial efímera de selección, etiquetas recomendadas y requisito de sesión de navegador. `observed_at` no aparece en sobres de error.

## Segunda forma: TUI para uso humano

`cineplanet-cli` sin argumentos, o `cineplanet-cli tui`, abre la interfaz
interactiva. Es útil cuando quieres explorar visualmente la cartelera y moverte
por las opciones con el teclado; no es la interfaz que deben automatizar los
agentes.

```bash
cineplanet-cli tui
```

**Cantidad de opciones vs tamaño de grupo.** El agente trata estas dos dimensiones como ejes independientes y las pasa como flags separadas:

- **Cantidad de opciones** — frases como `los 3 mejores`, `dame 3 opciones`, `muéstrame 5`, `los primeros 4`, `top N`, `N alternativas`, `N funciones` y `N horarios` se mapean a `--limit N` y **nunca** a `--party-size`. Interpretar `top 3` como grupo de tres es exactamente el bug que estamos corrigiendo.
- **Tamaño de grupo = 1** — `para mí`, `voy solo`, `voy sola`, `individual`, `una persona`, `yo nada más` se mapean a `--party-size 1`. No se asume 1 en su ausencia.
- **Tamaño de grupo inequívoco** — `somos tres`, `somos 4`, `para 3 personas`, `para cuatro`, `vamos 3`, `tres asientos juntos`, `4 amigos`, `mi familia de 5`, `en pareja`, `con mi novia`, `con mi esposo`, `N espacios`, `N lugares`, `N asientos`, `N butacas`, `N entradas`, `espacio para N`, `lugares para N` y `asientos para N` se mapean a `--party-size` con el conteo nombrado (2 para las frases en pareja `en pareja` / `con mi novia` / `con mi esposo`, porque implican dos personas). En este dominio, un número con semántica de asientos/personas es suficiente: `3 espacios buenos`, `4 entradas` y `para 3` significan grupos de 3, 4 y 3. Solo preguntar una vez cuando no exista ninguna señal de tamaño de grupo; nunca heredar el tamaño de grupo de un turno previo.

```json
{
  "version": "v1",
  "observed_at": "2026-08-15T12:34:56.789012+00:00",
  "recommendations": [{
    "rank": 1,
    "viewing": { "score": 99.7, "zone": { "id": "central_middle_rear" } },
    "selected_block": { "seats": [{ "row": "G", "number": 6 }, { "row": "G", "number": 7 }] },
    "seat_preview": {
      "screen": "PANTALLA",
      "symbols": { "available": ".", "occupied": "#", "accessible": "A", "recommended": "*", "aisle": " " },
      "rows": [{ "label": "G", "layout": ".. ..**#" }]
    },
    "checkout_handoff": {
      "movie_slug": "la-odisea", "cinema_id": "0000000007", "session_id": "66776",
      "seat_selection_url": "https://www.cineplanet.com.pe/compra/la-odisea/0000000007/66776/asientos",
      "selected_seat_labels": ["G6", "G7"],
      "session_fingerprint": "cineplanet:0000000007:66776", "browser_session_required": true
    }
  }],
  "diagnostics": { "map_failures": [] }
}
```
El sobre JSON `v1` se conserva verbatim — la transformación visual se aplica **solo** a la respuesta humana. La paleta humana es de ancho 2: `.` → `◻ ` disponible, `#` → `◼ ` ocupado, `A` → `♿`, `*` → `♟︎ ` recomendado y espacio → dos espacios. El recomendado usa la presentación de texto monocromática de una pequeña figura, por lo que no depende del soporte de colores y sigue diferenciándose de disponibles y ocupados. No se usa el emoji `🟧`, porque macOS lo dibuja más grande y con degradado. No se mezclan celdas ASCII con el mapa humano ni se usan cajas cerradas o bordes derechos. Las opciones se separan mediante una regla horizontal fuera del mapa.

| Celda JSON | Glifo | Significado   |
|-----------|-------|---------------|
| `.`       | ◻    | disponible    |
| `#`       | ◼    | ocupado       |
| `A`       | ♿    | accesibilidad |
| `*`       | ♟︎    | recomendado   |
| ` `       | `  ` (dos espacios) | pasillo / hueco |

La respuesta comienza con tres líneas compactas y cada opción usa una sección abierta `┌─ N` / `└─ Elegir: N`. Si la petición dice «en San Miguel o Salaverry», ambas sedes son filtros estrictos (`--venue`) y jamás aparecerá una sede distinta; solo palabras explícitas como «prefiere» o «prioriza» activan `--favorite-venue`. Por defecto #1, #2 y #3 muestran cada una su `seat_preview` completo en un bloque `text`, con todas las filas y columnas. Se preservan las filas vacías estructurales que expresan la geometría de la sala, sin añadir padding decorativo ajeno al mapa. Los mapas conservan la paleta documentada y no usan bordes, incluido el derecho; `mapa N` puede repetir la sala completa almacenada como detalle, sin revelar contenido oculto. En cualquier opción se muestran hora/sede, libres/modalidad, visión en español y los asientos explícitos, sin rangos. Ejemplo documental:

````text
🎬 LA ODISEA · HOY · NOCHE · 2 PERSONAS
📍 SOLO LA MOLINA + SALAVERRY · 3 OPCIONES
🕒 Actualizado 14:34 (Lima) · 12/12 mapas · 0 fallas

┌─ ⭐ 1 · 20:30 · CP LA MOLINA
  55 libres · 2D · SUB · Regular
  ♟︎ G6, G7 · Visión 97/100 Excelente · juntos · zona central media-trasera
  PANTALLA
  ```text
  E  ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  F  ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  G  ◻ ◻ ◻ ◼ ◻ ♟︎ ♟︎ ◻ ◻
  H  ◼ ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◼
  I  ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  ```
└─ Elegir: 1

────────────────────────────────────────────────────────────

┌─ 2 · 21:00 · CP SALAVERRY
  18 libres · 2D · SUB · PRIME
  ♟︎ G6, G7 · Visión 95/100 Excelente · juntos · zona central media-trasera
  PANTALLA
  ```text
  E  ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  F  ◻ ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻
  G  ◻ ◻ ◻ ◻ ◼ ♟︎ ♟︎ ◻ ◻
  H  ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◼ ◻
  I  ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  ```
└─ Elegir: 2

────────────────────────────────────────────────────────────

┌─ 3 · 21:30 · CP LA MOLINA
  27 libres · 2D · DOB · Regular
  ♟︎ H7, H8 · Visión 88/100 Buena · juntos · zona media
  PANTALLA
  ```text
  F  ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  G  ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻ ◼
  H  ◻ ◻ ◻ ◼ ◻ ♟︎ ♟︎ ◻ ◻
  I  ◻ ◻ ◼ ◻ ◻ ◻ ◻ ◻ ◻
  J  ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻ ◻
  ```
└─ Elegir: 3

◻ disponible · ◼ ocupado · ♿ accesibilidad · ♟︎ recomendado · dos espacios = pasillo / hueco
Disponibilidad observada a 2026-08-15T19:34:56+00:00; puede cambiar.
````


El contrato es deliberadamente pequeño: `recommend` y la TUI son de solo
lectura; solo `checkout --yes` crea una retención. Una función agotada se
reporta como tal y una falla parcial de mapa nunca se convierte en datos
inventados. La disponibilidad siempre incluye `observed_at` porque puede
cambiar antes de comprar.

## Stack

- **Rust**: CLI, dominio, ranking, parsing y concurrencia asíncrona.
- **Tokio + Reqwest**: consultas HTTP en vivo y reutilización de sesión.
- **Ratatui + Crossterm**: TUI multiplataforma para exploración humana.
- **Node.js + Playwright Core**: checkout visible y controlado en Chrome; no
  descarga otro navegador ni depende de Agent Browser.
- **JSON v1**: contrato estable para Codex, Claude Code y cualquier harness.
- **Cargo + npm**: distribución del binario y dependencia mínima del checkout.

## Roadmap

1. Modos de decisión: Mejor vista, Más pronto y Todos juntos.
2. Filtros por horario y modalidad.
3. Agrupación de estrenos.
4. Monitoreo técnico del contrato público de Cineplanet.

El runtime es determinista: no usa LLM. Las consultas normales son HTTP; el checkout explícito usa un Chrome visible y persistente mediante Playwright.
Al iniciar crea una sesión anónima efímera y la reutiliza durante esa ejecución. Si el contrato público cambia, la aplicación falla de forma explícita y nunca sustituye datos reales por demo. El demo solo se habilita explícitamente con `CINEPLANET_DEMO=1`.

## Instalación en macOS

Instala las Herramientas de línea de comandos de Xcode, Rust estable, Node.js 20+ y Google Chrome. Luego, desde un clon del repositorio:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/asther0/cineplanet-cli.git
cd cineplanet-cli
npm install
cargo install --path .
```

`npm install` instala `playwright-core`; no descarga otro navegador porque el checkout reutiliza Google Chrome. La TUI y `recommend` funcionan sin Playwright, pero `checkout` lo requiere.

Si `cargo` no está disponible tras instalar Rust, cierra y vuelve a abrir la terminal.

Luego inicia la aplicación con:

```bash
cineplanet-cli
```

Controles de la TUI:

- Flechas: moverte entre opciones.
- Escribir: filtrar al instante la lista visible.
- Backspace: borrar caracteres del filtro.
- Enter: seleccionar o continuar.
- Space: alternar selección en los selectores múltiples.
- Esc: volver al paso anterior.
- Q: salir.

macOS es la plataforma actualmente verificada.
