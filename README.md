# CineplanetCLI

![Rust](https://img.shields.io/badge/Rust-CE422B?style=flat&logo=rust&logoColor=white)
![Ratatui](https://img.shields.io/badge/Ratatui-FFC131?style=flat&logoColor=black)
![Crossterm](https://img.shields.io/badge/Crossterm-2D7DD2?style=flat&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=flat&logoColor=white)
![Reqwest](https://img.shields.io/badge/Reqwest-6E2EB1?style=flat&logoColor=white)

[![image.png](https://i.postimg.cc/nLpQTNqN/image.png)](https://postimg.cc/w14vjffk)

Encuentra funciones de Cineplanet con buenos asientos, sin revisar sede por sede.

```text
┌ CineplanetCLI ───────────────────────────────────────────┐
│ Película: Spider-Man                                    │
│ Grupo: 2 personas                                       │
├ Funciones disponibles ──────────────────────────────────┤
│ > 20:30  55 asientos  CP La Molina                       │
│   21:00  18 asientos  CP Salaverry                       │
└ ↑↓ elegir  Enter ver o modificar  Esc volver  Q salir ─┘
```

## Qué hace

- Consulta en vivo y de forma anónima la cartelera y los mapas de asientos públicos de Cineplanet.
- Guía la selección de ciudad, película, fecha, sede y grupo de 1-5 personas.
- Ordena las funciones cronológicamente e indica hora, asientos disponibles, sede y modalidad.
- Analiza cada función y etiqueta el ajuste para el grupo y la zona de visualización.
- Muestra un mapa de sala fiel a la distribución real y sus asientos observados.
- Recomienda el mejor bloque contiguo en la zona media o media-trasera, cerca del centro.
- Conserva las sedes favoritas como preferencias locales para la lógica de recomendaciones.
- Cuando el bloque cruza un pasillo, ofrece la alternativa disponible y explica la separación.
- La compra y la reserva siguen ocurriendo fuera de la CLI, en Cineplanet.

## Flujo

```text
Ciudad → película → fecha(s) → sede(s) → grupo (1-5) → funciones → mapa
```

La TUI permite escribir para filtrar ciudades, películas, fechas, sedes y funciones al instante. También funciona con flechas, Enter, Escape y selectores múltiples; no requiere memorizar comandos.

## TUI y recomendaciones para agentes

`cineplanet-cli` inicia la TUI por defecto. Para una consulta automatizada o desde Codex, usa el comando no interactivo `recommend`: escribe un único documento JSON versionado en stdout (o un error JSON versionado en stderr), sin abrir la TUI.

```bash
cineplanet-cli recommend \
  --movie-title "La Odisea" --city "Lima" --party-size 2 \
  --date 2026-08-15 --venue "CP La Molina" \
  --language Subtitulada --format 2D --room-type Regular \
  --favorite-venue la-molina --limit 3
```

El selector de película es exactamente uno de `--movie-id` o `--movie-title`; ciudad y grupo (`1` a `5`) son obligatorios. Repite `--date`, `--venue`, `--language`, `--format`, `--room-type` y `--favorite-venue` para varios valores. Las fechas usan `YYYY-MM-DD` en America/Lima. La respuesta `v1` contiene `observed_at` (RFC 3339 en UTC, momento en que la CLI observó la disponibilidad), `query`, recomendaciones ordenadas con `venue`, `starts_at`, `modality`, `viewing` (score, calidad, zona y razones), `selected_block`, y `diagnostics` (`candidate_count`, `hydrated_count`, `map_failures`). Cuando la fuente live entrega los IDs oficiales, cada recomendación añade `checkout_handoff` con el slug, sede, sesión, URL de selección, etiquetas de asientos y el requisito de sesión de navegador. `observed_at` no aparece en sobres de error.

```json
{
  "version": "v1",
  "observed_at": "2026-08-15T12:34:56.789012+00:00",
  "recommendations": [{
    "rank": 1,
    "viewing": { "score": 99.7, "zone": { "id": "central_middle_rear" } },
    "selected_block": { "seats": [{ "row": "G", "number": 6 }, { "row": "G", "number": 7 }] },
    "checkout_handoff": {
      "movie_slug": "la-odisea", "cinema_id": "0000000007", "session_id": "66776",
      "seat_selection_url": "https://www.cineplanet.com.pe/compra/la-odisea/0000000007/66776/asientos",
      "selected_seat_labels": ["G6", "G7"], "browser_session_required": true
    }
  }],
  "diagnostics": { "map_failures": [] }
}
```

En Codex, el skill repo-scoped `$cineplanet-recommend` se descubre desde `.agents/skills/cineplanet-recommend`; también se activa de forma natural con peticiones como “¿dónde veo La Odisea temprano en Lima para 3?”, “busca 2D subtitulada en San Miguel” o “encuéntrame cuatro asientos juntos”. El agente usa `recommend`, no la TUI, conserva el orden y la evaluación devueltos, y comunica que los asientos son observados y pueden cambiar. Por defecto solo recomienda. Ante una petición explícita como “continúa con la primera opción y retén los asientos”, vuelve a consultar, usa el navegador sobre la URL oficial, selecciona únicamente las etiquetas recomendadas y se detiene en `/entradas` tras **Seguir como invitado**. Es una asistencia de navegador: `add-tickets` está cifrado y la retención de aproximadamente cinco minutos depende de esa sesión; no es portable. Nunca elige categoría, promoción, confitería ni pago, y nunca envía un pago.

El comando filtra película, ciudad, fecha, sede y modalidad antes de hidratar mapas de sala; filtrar más reduce las consultas posteriores. Si Cineplanet cambia su contrato público, falla explícitamente y no inventa datos. Una falla parcial de mapas aparece en `diagnostics.map_failures`; si no se hidrata ningún candidato con fallas, devuelve un error JSON. La CLI por sí misma no inicia sesión ni reserva, retiene o compra entradas.

## Roadmap

1. Modos de decisión: Mejor vista, Más pronto y Todos juntos.
2. Revalidación de asientos antes de la entrega y opción de abrir o copiar el enlace de Cineplanet.
3. Filtros por horario y modalidad.
4. Agrupación de estrenos.
5. Monitoreo técnico del contrato público de Cineplanet.

El runtime es determinista: no usa LLM ni navegador oculto durante el uso normal.
Al iniciar crea una sesión anónima efímera y la reutiliza durante esa ejecución. Si el contrato público cambia, la aplicación falla de forma explícita y nunca sustituye datos reales por demo. El demo solo se habilita explícitamente con `CINEPLANET_DEMO=1`.

## Instalación en macOS

Instala las Herramientas de línea de comandos de Xcode y Rust estable. Luego, desde un clon del repositorio:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/asther0/cineplanet-cli.git
cd cineplanet-cli
cargo install --path .
```

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
