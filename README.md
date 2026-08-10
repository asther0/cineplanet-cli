# CineplanetCLI

Encuentra funciones de Cineplanet con buenos asientos, sin revisar sede por sede.

```text
┌ CineplanetCLI ───────────────────────────────────────────┐
│ Película: Spider-Man                                    │
│ Grupo: 2 personas                                       │
├ Mejores funciones ──────────────────────────────────────┤
│ > Excelente  CP La Molina   Hoy 20:30   2D Subtitulada  │
│   Buena      CP Alcázar     Mañana 19:30   2D           │
│   Buena      CP Salaverry   Martes 21:00   Prime        │
└ ↑↓ elegir  Enter abrir  F filtros  Q salir ─────────────┘
```

## Qué hace

- Recorre la cartelera publicada por Cineplanet.
- Compara sedes, fechas, horarios y modalidades.
- Busca bloques contiguos cerca del centro y la zona media-trasera.
- Explica por qué recomienda cada función.
- Mantiene la compra y la reserva en la web de Cineplanet.

## Flujo

```text
Película → preferencias → análisis → top 3 → mapa de sala → Cineplanet
```

La experiencia será una TUI navegable con flechas, Enter, Escape y selectores múltiples. No requerirá memorizar comandos.

## Estado

En desarrollo temprano.

- [x] Dominio de funciones, preferencias y mapas de sala
- [x] Primer ranking de bloques contiguos
- [x] Preferencias persistentes y onboarding de sedes
- [x] Flujo TUI completo con datos demo
- [ ] Adaptador HTTP de Cineplanet
- [ ] Filtros interactivos de fecha, hora y modalidad
- [ ] Revalidación y entrega a Cineplanet

El runtime será determinista: sin LLM y sin navegador oculto durante el uso normal.
Hasta conectar el adaptador HTTP, la aplicación muestra `MODO DEMO` de forma permanente.

## Desarrollo

Requiere Rust estable.

```bash
cargo test
cargo run
```

El MVP tendrá soporte verificado en macOS.
