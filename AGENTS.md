# AGENTS.md — side/CineplanetCLI

Este proyecto de código está enlazado al vault Obsidian de asther0. La carpeta
de Proyecto en el vault vive en:

/Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/30 projects/side/CineplanetCLI

## Punteros al vault (leer al iniciar sesión)

CRITICAL: Antes de actuar en este repo, lee con tu Read tool los siguientes
archivos. Son **instrucciones obligatorias** y pisan los defaults:

- @.vault-context/AGENTS-vault.md   — contrato de agente vault-wide
- @.vault-context/CONTEXT-vault.md  — glosario vault-wide (lenguaje ubicuo)
- @.vault-context/commit-style.md   — estilo universal de commits (micro-atomic, Conventional)
- @.vault-context/recommended-agent-skills.md — skills recomendadas para este Proyecto
- @.vault-context/herdr-agent-orchestration.md — coordinación de agentes y propiedad
- @.vault-context/model-routing.md — roles, modelos, costes y dispatch de Herdr

Además, lee la documentación específica de este Proyecto directamente del vault
(la ruta ya está permitida en opencode.json → permission.external_directory):

- /Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/30 projects/side/CineplanetCLI/CONTEXT.md  — glosario del dominio side/CineplanetCLI
- /Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/30 projects/side/CineplanetCLI/README.md   — estado actual + Avances recientes (dónde quedó el proyecto)

No apuntes a estos dos con `@ file` porque viven fuera del worktree (no son
symlinks): tu Read tool debe abrir las rutas absolutas directamente.

## Regla dura: fuente única

`.vault-context/` es la **única** carpeta de punteros al vault en este repo.

- NO crees archivos `AGENTS.md` en subdirectorios de este repo.
- NO crees archivos `CONTEXT.md` en este repo.
- NO crees symlinks adicionales al vault fuera de `.vault-context/`.
- Cualquier instrucción específica vive en el `AGENTS.md` de la raíz del
  repo (este archivo) o en el vault mismo. Si necesitás una referencia
  nueva al vault, añadí el symlink al script
  `/Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/00 meta/bin/init-vault-context.sh` y volvé a correrlo —
  no improvises symlinks manuales.

## Cómo trabajar

- Commits: aplicar `commit-style.md` sin excepción (micro-atomic, Conventional).
- Decisiones irreversibles + sorprendentes + con alternativas reales → ADR.
  Ver AGENTS-vault.md. Propón, no escribas solo.
- Al cerrar un PR con aprendizaje/decision no trivial, sigue el flujo de
  "Avances recientes" descrito en AGENTS-vault.md (actualizar
  `/Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/30 projects/side/CineplanetCLI/README.md` del vault, opcionalmente crear retro). NO
  escribas en `.vault-context/` — es read-only.

## No escribir en el vault sin permiso

Los archivos en `.vault-context/` son punteros read-only por symlink. Si tu
cambio requiere modificar el vault (estado del proyecto, nueva retro, nuevo
ADR), pide confirmación al usuario antes de escribir en:

/Users/asther0/Desktop/JHOMAR_LAB/asther0 vault/30 projects/side/CineplanetCLI/

Y nunca escribas en otras carpetas del vault (`10 second-brain/`,
`50 people/`, `CONTEXT.md` raíz) sin confirmación explícita (ver
AGENTS-vault.md).
