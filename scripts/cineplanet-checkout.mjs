import { execFileSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { pathToFileURL } from "node:url";

const options = parseArgs(process.argv.slice(2));
const required = ["url", "cinema", "session", "fingerprint", "seats", "profile", "playwright"];
for (const name of required) {
  if (!options[name]) fail(`Falta --${name}.`);
}

const url = new URL(options.url);
const expectedPath = new RegExp(
  `^/compra/[^/]+/${escapeRegExp(options.cinema)}/${escapeRegExp(options.session)}/asientos/?$`,
);
if (url.protocol !== "https:" || url.hostname !== "www.cineplanet.com.pe" || !expectedPath.test(url.pathname)) {
  fail("La URL de selección no coincide con la sede y sesión revalidadas.");
}
if (options.fingerprint !== `cineplanet:${options.cinema}:${options.session}`) {
  fail("La huella de sesión no coincide con la sede y sesión revalidadas.");
}

const requestedSeats = JSON.parse(options.seats);
if (!Array.isArray(requestedSeats) || requestedSeats.length === 0 || requestedSeats.some((seat) => typeof seat !== "string")) {
  fail("--seats debe contener un arreglo JSON no vacío de etiquetas.");
}

const { chromium } = await import(pathToFileURL(options.playwright).href);
const port = Number.parseInt(options.port || "9224", 10);
if (!Number.isInteger(port) || port < 1024 || port > 65535) fail("Puerto CDP inválido.");

await ensureChrome(port, options.profile);
const browser = await connectToChrome(chromium, port, options.profile);
const context = browser.contexts()[0];
if (!context) fail("Chrome no expuso un contexto persistente.");
await closeStaleCheckoutPages(context, options.cinema, options.session);
const page = await context.newPage();
const apiTrace = captureCheckoutApiTrace(page);
const cdp = await context.newCDPSession(page);
for (const name of ["geolocation", "notifications"]) {
  await cdp.send("Browser.setPermission", {
    origin: "https://www.cineplanet.com.pe",
    permission: { name },
    setting: "denied",
  });
}

try {
  let guestWasEstablishedAfterSelection = false;
  await establishGuestSession(page);
  const seatPlanPath = `/api/v1-web/seatplan/cinema/${options.cinema}/session/${options.session}`;
  const [seatPlanHttpResponse] = await Promise.all([
    page.waitForResponse(
      (response) => new URL(response.url()).pathname === seatPlanPath && response.request().method() === "GET",
      { timeout: 30_000 },
    ),
    page.goto(url.href, { waitUntil: "domcontentloaded", timeout: 30_000 }),
  ]);
  await page.locator(".seat-map--seat").first().waitFor({ timeout: 30_000 });
  await acceptCookies(page, 150);
  await clearExistingSelection(page);
  if (!seatPlanHttpResponse.ok()) {
    fail(`seatplan respondió HTTP ${seatPlanHttpResponse.status()}`);
  }
  const seatPlan = await seatPlanHttpResponse.json();

  const officialSeats = normalizeSeatPlan(seatPlan);
  const controls = await page
    .locator(".seat-map--seat:not(.seat-map--seat_broken):not(.seat-map--seat_companion)")
    .evaluateAll((elements) =>
    elements.map((element, index) => {
      const rect = element.getBoundingClientRect();
      return { index, x: rect.x, y: rect.y, className: element.className };
    }),
  );
  const orderedControls = sortSeatControls(controls);

  if (officialSeats.length !== orderedControls.length) {
    const classCounts = Object.fromEntries(
      [...new Set(orderedControls.map((control) => control.className))]
        .sort()
        .map((className) => [className, orderedControls.filter((control) => control.className === className).length]),
    );
    fail(`El mapa oficial tiene ${officialSeats.length} butacas, pero la web dibujó ${orderedControls.length}. Clases DOM: ${JSON.stringify(classCounts)}. No se hizo clic.`);
  }

  const mapped = new Map();
  for (let index = 0; index < officialSeats.length; index += 1) {
    const seat = officialSeats[index];
    const control = orderedControls[index];
    const renderedStatus = domSeatStatus(control.className);
    if (renderedStatus !== seat.status) {
      fail(`La web dibujó ${seat.label} con estado ${renderedStatus}, pero seatplan indicó ${seat.status}. No se hizo clic.`);
    }
    mapped.set(seat.label, {
      ...seat,
      controlIndex: control.index,
      domX: control.x,
      domY: control.y,
    });
  }

  const targets = requestedSeats.map((label) => {
    const target = mapped.get(label);
    if (!target) fail(`La web no contiene la butaca revalidada ${label}. No se hizo clic.`);
    if (target.status !== 0) fail(`La butaca ${label} ya no está disponible. No se hizo clic.`);
    if (!orderedControls.find((control) => control.index === target.controlIndex)?.className.includes("seat-map--seat_available")) {
      fail(`La web ya no muestra ${label} como disponible. No se hizo clic.`);
    }
    return target;
  });

  if (options["dry-run"] === "true") {
    emit({
      version: "v1",
      status: "validated",
      selected_seats: requestedSeats,
      seat_selection_url: page.url(),
      seat_count: officialSeats.length,
      dom_classes: [...new Set(orderedControls.map((control) => control.className))].sort(),
      selected_positions: targets.map(({ label, status, x, y, controlIndex, domX, domY }) => ({
        label,
        status,
        x,
        y,
        control_index: controlIndex,
        dom_x: domX,
        dom_y: domY,
      })),
    });
    process.exit(0);
  }

  for (const target of targets) {
    const control = page
      .locator(".seat-map--seat:not(.seat-map--seat_broken):not(.seat-map--seat_companion)")
      .nth(target.controlIndex);
    await control.click({ timeout: 10_000 });
    const className = await control.getAttribute("class");
    if (!className?.includes("seat-map--seat_selected")) {
      fail(`Cineplanet no confirmó la selección de ${target.label}.`);
    }
  }

  const selectedCount = await page.locator(".seat-map--seat.seat-map--seat_selected").count();
  if (selectedCount !== requestedSeats.length) {
    fail(`Cineplanet confirmó ${selectedCount} butacas, se esperaban ${requestedSeats.length}.`);
  }
  const continueButton = page
    .locator("button.submit-button--button:not([disabled])")
    .filter({ hasText: "Continuar" })
    .first();
  await continueButton.click({ timeout: 10_000 });
  await page.waitForURL(
    (current) => current.pathname === "/autenticacion/login" || current.pathname.endsWith("/entradas"),
    { timeout: 30_000 },
  );
  if (new URL(page.url()).pathname === "/autenticacion/login") {
    await page.getByRole("button", { name: "Seguir como invitado", exact: true }).click({ timeout: 10_000 });
    guestWasEstablishedAfterSelection = true;
  }
  await page.waitForURL("**/entradas", { waitUntil: "domcontentloaded", timeout: 30_000 });

  if (!(await checkoutIsReady(page, requestedSeats.length))) {
    await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
    if (!(await checkoutIsReady(page, requestedSeats.length))) {
      if (guestWasEstablishedAfterSelection) {
        fail("guest_session_initialized: Cineplanet creó una nueva sesión invitada; hay que revalidar y seleccionar nuevamente.");
      }
      fail(`La página /entradas no confirmó las butacas y tarifas. Respuestas relevantes: ${JSON.stringify(apiTrace)}. La pestaña quedó abierta para revisión.`);
    }
  }

  await page.bringToFront();
  activateChrome();
  emit({
    version: "v1",
    status: "ready_for_ticket_selection",
    checkout_url: page.url(),
    selected_seats: requestedSeats,
    hold_expires_approx_seconds: 300,
    browser_session_required: true,
  });
  process.exit(0);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}

function sortSeatControls(controls) {
  const rowTolerancePx = 2;
  const byVerticalPosition = [...controls].sort((left, right) => left.y - right.y || left.x - right.x);
  const rows = [];
  for (const control of byVerticalPosition) {
    const row = rows.find((candidate) => Math.abs(candidate.anchorY - control.y) <= rowTolerancePx);
    if (row) {
      row.controls.push(control);
      row.anchorY = row.controls.reduce((sum, seat) => sum + seat.y, 0) / row.controls.length;
    } else {
      rows.push({ anchorY: control.y, controls: [control] });
    }
  }
  rows.sort((left, right) => left.anchorY - right.anchorY);
  return rows.flatMap((row) => row.controls.sort((left, right) => left.x - right.x));
}

function domSeatStatus(className) {
  if (className.includes("seat-map--seat_available")) return 0;
  if (className.includes("seat-map--seat_ocupied")) return 1;
  if (className.includes("seat-map--seat_special")) return 3;
  return null;
}

async function establishGuestSession(page) {
  if (await guestSessionIsFresh(page.context())) return;
  await page.goto("https://www.cineplanet.com.pe/autenticacion/login", {
    waitUntil: "domcontentloaded",
    timeout: 30_000,
  });
  await acceptCookies(page, 1_500);
  const guest = page.getByRole("button", { name: "Seguir como invitado", exact: true });
  if (!(await guest.isVisible({ timeout: 2_000 }).catch(() => false))) return;
  await guest.click({ timeout: 10_000 });
  await page.waitForURL((current) => current.pathname !== "/autenticacion/login", { timeout: 30_000 });
}

async function guestSessionIsFresh(context) {
  const cookies = await context.cookies("https://www.cineplanet.com.pe");
  const session = cookies.find((cookie) => cookie.name === "userSessionId" && cookie.value);
  return Boolean(session && session.expires > Date.now() / 1_000 + 30);
}

async function closeStaleCheckoutPages(context, cinema, session) {
  const fragment = `/compra/`;
  const identity = `/${cinema}/${session}/`;
  for (const stale of context.pages()) {
    const staleUrl = stale.url();
    if (staleUrl.includes(fragment) && staleUrl.includes(identity)) {
      await stale.close().catch(() => {});
    }
  }
}

function captureCheckoutApiTrace(page) {
  const trace = [];
  page.on("response", async (response) => {
    let parsed;
    try {
      parsed = new URL(response.url());
    } catch {
      return;
    }
    if (parsed.hostname !== "www.cineplanet.com.pe" || !parsed.pathname.startsWith("/api/v1-web/")) return;
    if (!/(add-tickets|get-order|gettickets|seatplan|guest)/i.test(parsed.pathname)) return;
    let summary = "";
    try {
      const body = await response.text();
      summary = body.includes('"encResponse"')
        ? "<respuesta cifrada recibida>"
        : body.replace(/\s+/g, " ").slice(0, 500);
    } catch {
      // Some navigations dispose the response body before Playwright can read it.
    }
    trace.push({
      method: response.request().method(),
      path: parsed.pathname,
      status: response.status(),
      summary,
    });
  });
  return trace;
}

async function checkoutIsReady(page, partySize) {
  await page.waitForFunction(
    (expected) => {
      const text = document.body?.innerText || "";
      const count = new RegExp(`Entradas seleccionadas:\\s*0\\s+de\\s+${expected}`, "i").test(text)
        || new RegExp(`${expected}\\s+Butacas? seleccionadas?`, "i").test(text);
      return count && text.toLocaleLowerCase("es").includes("entradas generales");
    },
    partySize,
    { timeout: 8_000 },
  ).catch(() => {});
  const body = await page.locator("body").innerText();
  const countIsVisible = new RegExp(`Entradas seleccionadas:\\s*0\\s+de\\s+${partySize}`, "i").test(body)
    || new RegExp(`${partySize}\\s+Butacas? seleccionadas?`, "i").test(body);
  const generalIndex = body.toLocaleLowerCase("es").indexOf("entradas generales");
  if (!countIsVisible || generalIndex < 0) return false;
  const nearby = body.slice(generalIndex, generalIndex + 2_500);
  return /S\/\s*\d|general|adult/i.test(nearby) && (await page.locator("button").count()) > 2;
}

function normalizeSeatPlan(response) {
  if (response.ResponseCode !== "0" || !response.SeatLayoutData?.Areas) {
    fail(`Cineplanet no entregó el mapa oficial: ${response.ErrorDescription || response.ResponseCode}`);
  }
  const areas = [...response.SeatLayoutData.Areas].sort(
    (left, right) => (left.Top || 0) - (right.Top || 0)
      || (left.Left || 0) - (right.Left || 0)
      || (left.Number || 0) - (right.Number || 0),
  );
  const result = [];
  let yOffset = 0;
  for (const area of areas) {
    const all = (area.Rows || []).flatMap((row) => row.Seats || []);
    const rows = Math.max(area.RowCount || 0, ...all.map((seat) => (seat.Position?.RowIndex || 0) + 1), 0);
    const columns = Math.max(area.ColumnCount || 0, ...all.map((seat) => (seat.Position?.ColumnIndex || 0) + 1), 0);
    for (const row of area.Rows || []) {
      const rowName = row.PhysicalName;
      if (!rowName) continue;
      for (const seat of row.Seats || []) {
        if (![0, 1, 3].includes(seat.Status)) continue;
        result.push({
          label: `${rowName}${seat.Id}`,
          status: seat.Status,
          x: columns - 1 - seat.Position.ColumnIndex,
          y: yOffset + rows - 1 - seat.Position.RowIndex,
        });
      }
    }
    yOffset += rows + 1;
  }
  return result.sort((left, right) => left.y - right.y || left.x - right.x || left.label.localeCompare(right.label));
}

async function acceptCookies(page, waitMs) {
  const overlay = page.locator(".consent--background:visible");
  const button = page.locator("button:visible").filter({ hasText: "Aceptar Cookies" }).last();
  const appeared = await button.waitFor({ state: "visible", timeout: waitMs }).then(() => true).catch(() => false);
  if (!appeared && !(await overlay.count())) return;
  if (appeared) await button.click({ timeout: 2_000 }).catch(() => {});
  await overlay.waitFor({ state: "hidden", timeout: 2_000 }).catch(() => {});
  if (await overlay.count()) {
    fail("No se pudo cerrar el consentimiento de cookies. No se seleccionaron butacas.");
  }
}

async function clearExistingSelection(page) {
  for (let attempt = 0; attempt < 8; attempt += 1) {
    const selected = page.locator(".seat-map--seat.seat-map--seat_selected");
    if (!(await selected.count())) return;
    await selected.first().click({ timeout: 5_000 });
    await page.waitForTimeout(100);
  }
  const remaining = await page.locator(".seat-map--seat.seat-map--seat_selected").count();
  if (remaining !== 0) {
    fail(`No se pudo limpiar la selección previa (${remaining} restantes).`);
  }
}

async function ensureChrome(port, profile) {
  if (await cdpEndpoint(port)) return;
  const executable = chromeExecutable();
  const child = spawn(executable, [
    `--remote-debugging-port=${port}`,
    "--remote-debugging-address=127.0.0.1",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--deny-permission-prompts",
    "--disable-features=Translate",
    "about:blank",
  ], { detached: true, stdio: "ignore" });
  child.unref();
  for (let attempt = 0; attempt < 60; attempt += 1) {
    if (await cdpEndpoint(port)) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  fail("Chrome no habilitó el puerto local de automatización.");
}

async function cdpEndpoint(port) {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/json/version`);
    if (!response.ok) return null;
    const metadata = await response.json();
    if (!String(metadata.Browser || "").startsWith("Chrome/")) return null;
    return typeof metadata.webSocketDebuggerUrl === "string"
      ? metadata.webSocketDebuggerUrl
      : null;
  } catch {
    return null;
  }
}

async function connectToChrome(chromium, port, profile) {
  let lastError = "el endpoint CDP no estuvo disponible";
  let restarted = false;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const endpoint = await cdpEndpoint(port);
      if (endpoint) return await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
      if (!restarted && lastError.includes("Browser context management is not supported")) {
        await restartDedicatedChrome(port, profile);
        await ensureChrome(port, profile);
        restarted = true;
        continue;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  fail(`Playwright no pudo conectarse al Chrome persistente después de 15 s. Último error: ${lastError}`);
}

async function restartDedicatedChrome(port, profile) {
  const executable = chromeExecutable();
  const portFlag = `--remote-debugging-port=${port}`;
  const profileFlag = `--user-data-dir=${profile}`;
  const processes = execFileSync("ps", ["-axo", "pid=,command="], { encoding: "utf8" });
  const processLine = processes.split("\n").find((line) => {
    const command = line.trim().replace(/^\d+\s+/, "");
    return command.startsWith(`${executable} `)
      && command.includes(portFlag)
      && command.includes(profileFlag);
  });
  if (!processLine) {
    fail("Chrome rechazó la gestión del contexto y no se encontró el proceso dedicado para reiniciarlo con seguridad.");
  }
  const pid = Number.parseInt(processLine.trim(), 10);
  if (!Number.isInteger(pid) || pid <= 1) {
    fail("No se pudo identificar de forma segura el proceso dedicado de Chrome.");
  }
  process.kill(pid, "SIGTERM");
  for (let attempt = 0; attempt < 40; attempt += 1) {
    if (!(await cdpEndpoint(port))) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  fail("El Chrome dedicado no terminó a tiempo para reiniciarlo de forma segura.");
}

function chromeExecutable() {
  const candidates = [
    process.env.CINEPLANET_CHROME,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    `${process.env.HOME}/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`,
  ].filter(Boolean);
  const executable = candidates.find(existsSync);
  if (!executable) fail("No se encontró Google Chrome. Define CINEPLANET_CHROME con su ruta.");
  return executable;
}

function activateChrome() {
  if (process.platform !== "darwin") return;
  const child = spawn("open", ["-a", "Google Chrome"], { detached: true, stdio: "ignore" });
  child.unref();
}

function parseArgs(values) {
  const result = {};
  for (let index = 0; index < values.length; index += 2) {
    const key = values[index]?.replace(/^--/, "");
    const value = values[index + 1];
    if (!key || value === undefined) fail(`Argumento inválido: ${values[index] || "(vacío)"}`);
    result[key] = value;
  }
  return result;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function emit(value) {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

function fail(message) {
  process.stderr.write(`${JSON.stringify({ version: "v1", error: { kind: "checkout_failed", message } }, null, 2)}\n`);
  process.exit(1);
}
