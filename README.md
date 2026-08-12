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

- Recorre la cartelera publicada por Cineplanet.
- Lista las funciones consultadas con sus asientos disponibles observados.
- Busca bloques contiguos cerca del centro y la zona media-trasera.
- Explica por qué recomienda cada función.
- Mantiene la compra y la reserva en la web de Cineplanet.

## Flujo

```text
Película → fecha(s) → sede(s) → grupo (1–5) → confirmar búsqueda → funciones → mapa
```

La experiencia será una TUI navegable con flechas, Enter, Escape y selectores múltiples. No requerirá memorizar comandos.

## Estado

En desarrollo temprano.

- [x] Dominio de funciones, preferencias y mapas de sala
- [x] Primer ranking de bloques contiguos
- [x] Preferencias persistentes y onboarding de sedes
- [x] Adaptador HTTP público de Cineplanet: cartelera, sedes, funciones y mapas
- [x] Flujo TUI interactivo con pantalla de bienvenida
- [x] Selección guiada de fechas, sedes y grupo antes de consultar asientos
- [x] Alternativas con asientos disponibles cuando no hay bloque contiguo
- [ ] Filtros de horario y modalidad
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
