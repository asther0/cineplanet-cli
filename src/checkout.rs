use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::recommendation::CheckoutHandoffV1;

const RUNNER: &str = include_str!("../scripts/cineplanet-checkout.mjs");

#[derive(Debug, Deserialize)]
struct BrowserCheckoutResult {
    status: String,
    checkout_url: String,
    selected_seats: Vec<String>,
    hold_expires_approx_seconds: u64,
    browser_session_required: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponseV1 {
    pub version: &'static str,
    pub status: String,
    pub recommendation_id: String,
    pub venue: String,
    pub starts_at: String,
    pub checkout_url: String,
    pub selected_seats: Vec<String>,
    pub hold_expires_approx_seconds: u64,
    pub browser_session_required: bool,
}

pub fn open_guest_checkout(
    recommendation_id: String,
    venue: String,
    starts_at: String,
    handoff: &CheckoutHandoffV1,
) -> Result<CheckoutResponseV1> {
    ensure_macos()?;
    let node = resolve_node()?;
    let playwright = resolve_playwright()?;
    let project = ProjectDirs::from("dev", "asther0", "CineplanetCLI")
        .context("no se pudo resolver la carpeta local de CineplanetCLI")?;
    let runner = materialize_runner(project.cache_dir())?;
    let profile = project.data_local_dir().join("browser-profile");
    fs::create_dir_all(&profile).context("no se pudo crear el perfil persistente de Chrome")?;
    let seats = serde_json::to_string(&handoff.selected_seat_labels)?;

    let output = Command::new(node)
        .arg(runner)
        .args(["--url", &handoff.seat_selection_url])
        .args(["--cinema", &handoff.cinema_id])
        .args(["--session", &handoff.session_id])
        .args(["--fingerprint", &handoff.session_fingerprint])
        .args(["--seats", &seats])
        .arg("--profile")
        .arg(profile)
        .arg("--playwright")
        .arg(playwright)
        .output()
        .context("no se pudo iniciar el runner Playwright")?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(if message.is_empty() {
            "Playwright no pudo completar el checkout".to_owned()
        } else {
            message
        });
    }

    let browser: BrowserCheckoutResult = serde_json::from_slice(&output.stdout)
        .context("el runner Playwright devolvió una respuesta inválida")?;
    if browser.selected_seats != handoff.selected_seat_labels {
        bail!("Playwright no confirmó exactamente las butacas revalidadas");
    }
    Ok(CheckoutResponseV1 {
        version: "v1",
        status: browser.status,
        recommendation_id,
        venue,
        starts_at,
        checkout_url: browser.checkout_url,
        selected_seats: browser.selected_seats,
        hold_expires_approx_seconds: browser.hold_expires_approx_seconds,
        browser_session_required: browser.browser_session_required,
    })
}

fn ensure_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        bail!("el checkout Playwright está disponible inicialmente solo en macOS")
    }
}

fn materialize_runner(cache_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).context("no se pudo crear la caché de CineplanetCLI")?;
    let path = cache_dir.join("cineplanet-checkout.mjs");
    fs::write(&path, RUNNER).context("no se pudo preparar el runner Playwright")?;
    Ok(path)
}

fn resolve_node() -> Result<PathBuf> {
    let candidates = [
        std::env::var_os("CINEPLANET_NODE").map(PathBuf::from),
        Some(PathBuf::from("/opt/homebrew/bin/node")),
        Some(PathBuf::from("/usr/local/bin/node")),
        Some(PathBuf::from("node")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
        .context("no se encontró Node.js 20+; define CINEPLANET_NODE con su ruta")
}

fn resolve_playwright() -> Result<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        std::env::var_os("CINEPLANET_PLAYWRIGHT_CORE").map(PathBuf::from),
        std::env::current_dir()
            .ok()
            .map(|path| path.join("node_modules/playwright-core/index.mjs")),
        Some(manifest.join("node_modules/playwright-core/index.mjs")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.is_file())
        .context("falta playwright-core; ejecuta `npm install` en el repositorio CineplanetCLI")
}
