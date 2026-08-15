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
