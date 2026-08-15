# AGENTS.md — CineplanetCLI

## Alcance del proyecto

Este es un CLI de Rust para consultar información de Cineplanet.

## Consultas de Cineplanet

Para solicitudes en lenguaje natural que sean de solo lectura (películas,
funciones o asientos), usa la skill local
`.agents/skills/cineplanet-recommend/SKILL.md`. Ejecuta directamente el comando
determinista indicado por la skill; no uses Herdr ni subagentes para estas
consultas.

## Cambios de código

- Conserva los cambios ajenos existentes en el árbol de trabajo.
- Ejecuta `cargo fmt` cuando modifiques código Rust y `cargo check` cuando sea
  pertinente al cambio.
- Cuando se autoricen commits, usa commits Conventional micro-atómicos.
- No hagas commit ni push sin autorización explícita.

## Límites de archivos

- No escribas fuera de este repositorio ni en vaults, salvo petición explícita
  del usuario.
- No crees archivos `AGENTS.md` ni `CONTEXT.md` en subdirectorios.
- `.vault-context/` es una integración local opcional, ignorada por Git. Solo
  léela si el usuario pide explícitamente sincronizar o usar el vault o Herdr.
