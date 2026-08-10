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
- [x] Adaptador HTTP público de Cineplanet: cartelera, sedes, funciones y mapas
- [x] Flujo TUI interactivo con pantalla de bienvenida
- [ ] Filtros interactivos de fecha, hora y modalidad
- [ ] Revalidación y entrega a Cineplanet

El runtime será determinista: sin LLM y sin navegador oculto durante el uso normal.
Al iniciar, consulta el contrato público de Cineplanet con una sesión anónima efímera.
Si el contrato cambia, falla de forma explícita; nunca sustituye datos reales por demo.
El demo solo se habilita explícitamente con `CINEPLANET_DEMO=1`.

## Inicio

Tras instalar el binario, lanza la aplicación interactiva con un solo comando:

```bash
cineplanet-cli
```

La TUI responde al teclado:

- Flechas: moverte entre opciones.
- Enter: seleccionar o continuar.
- Space: alternar selección en los selectores múltiples.
- Esc: volver al paso anterior.
- Q: salir.

El MVP tendrá soporte verificado en macOS.
