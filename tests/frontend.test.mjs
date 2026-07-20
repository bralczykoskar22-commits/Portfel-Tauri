import assert from "node:assert/strict";
import fs from "node:fs/promises";
import test from "node:test";
import { Window } from "happy-dom";

const root = new URL("../", import.meta.url);
const html = await fs.readFile(new URL("src/index.html", root), "utf8");
const bridgeScript = await fs.readFile(new URL("src/assets/tauri-bridge.js", root), "utf8");
const appScript = await fs.readFile(new URL("src/assets/app.js", root), "utf8");
const initialData = JSON.parse(await fs.readFile(new URL("src/data/budget.json", root), "utf8"));

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

async function waitFor(predicate, message) {
  const deadline = Date.now() + 4000;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  throw new Error(message);
}

function setValue(window, selector, value, eventName) {
  const element = window.document.querySelector(selector);
  assert.ok(element, `Brak elementu ${selector}`);
  element.value = String(value);
  if (eventName) element.dispatchEvent(new window.Event(eventName, { bubbles: true }));
  return element;
}

function submit(window, selector) {
  const form = window.document.querySelector(selector);
  assert.ok(form, `Brak formularza ${selector}`);
  form.dispatchEvent(new window.Event("submit", { bubbles: true, cancelable: true }));
}

test("pełny interfejs Tauri działa przez natywne polecenia", async () => {
  const window = new Window({ url: "tauri://localhost/" });
  const calls = [];
  const snapshots = [];
  let stored = clone(initialData);
  let activeProfile = null;
  let profiles = [{
    id: "profile-default", name: "Mój profil", color: "#2563eb", avatarData: null,
    hasPassword: false, isDefault: true, createdAt: "2026-07-20T10:00:00.000Z", updatedAt: "2026-07-20T10:00:00.000Z"
  }];

  window.confirm = () => true;
  window.print = () => {};
  window.__TAURI__ = {
    core: {
      invoke: async (command, args = {}) => {
        calls.push({ command, args: clone(args) });
        if (command === "list_profiles") return clone(profiles);
        if (command === "login_profile") {
          const profile = profiles.find(item => item.id === args.options.profileId);
          if (!profile) throw new Error("Nie znaleziono profilu.");
          activeProfile = profile.id;
          return clone(profile);
        }
        if (command === "logout_profile") { activeProfile = null; return true; }
        if (command === "create_profile") {
          const profile = {
            id: "profile-new", name: args.options.name, color: args.options.color, avatarData: args.options.avatarData || null,
            hasPassword: !!args.options.password, isDefault: !!args.options.isDefault, createdAt: "2026-07-20T11:00:00.000Z", updatedAt: "2026-07-20T11:00:00.000Z"
          };
          if (profile.isDefault) profiles = profiles.map(item => ({ ...item, isDefault: false }));
          profiles.push(profile); activeProfile = profile.id; return clone(profile);
        }
        if (command === "update_profile") {
          profiles = profiles.map(item => item.id === activeProfile ? {
            ...item, name: args.options.name, color: args.options.color, avatarData: args.options.avatarData || null,
            hasPassword: args.options.removePassword ? false : (args.options.password ? true : item.hasPassword),
            isDefault: args.options.isDefault || item.isDefault
          } : (args.options.isDefault ? { ...item, isDefault: false } : item));
          return clone(profiles.find(item => item.id === activeProfile));
        }
        if (command === "load_data") {
          if (!activeProfile) throw new Error("Najpierw wybierz profil użytkownika.");
          return clone(stored);
        }
        if (command === "save_data") {
          snapshots.push(clone(stored));
          stored = clone(args.payload);
          stored.version = 6;
          stored.meta.savedAt = "2026-07-20T12:00:00.000Z";
          return {
            ok: true,
            savedAt: stored.meta.savedAt,
            path: "C:\\Users\\Test\\AppData\\Roaming\\Portfel\\data\\portfel.sqlite",
            backup: "portfel-20260720-120000-000.json"
          };
        }
        if (command === "list_backups") return { ok: true, backups: [] };
        if (command === "app_info") {
          return {
            version: "0.8.0-alpha.1",
            dataPath: "C:\\Users\\Test\\AppData\\Roaming\\Portfel\\data\\portfel.sqlite",
            dataDirectory: "C:\\Users\\Test\\AppData\\Roaming\\Portfel\\data",
            platform: "win32",
            packaged: true
          };
        }
        if (command === "updater_status" || command === "updater_check" || command === "updater_install") {
          return {
            state: "disabled",
            message: "Kanał aktualizacji zostanie aktywowany przy publikacji programu."
          };
        }
        if (command === "preview_statement") {
          return {
            ok: true,
            fileName: "konto.csv",
            delimiter: ";",
            headers: ["Data operacji", "Opis", "Kwota"],
            rows: [],
            skipped: 0,
            warnings: []
          };
        }
        if (command === "export_data") return { ok: true, path: "C:\\kopia.json" };
        if (command === "import_data") return { ok: false, canceled: true };
        if (command === "restore_backup") return { ok: true, restored: args.name };
        if (command === "open_data_folder") return "";
        throw new Error(`Nieobsługiwane polecenie ${command}`);
      }
    }
  };

  const markup = html
    .replace(/<link[^>]+rel="stylesheet"[^>]*>/g, "")
    .replace(/<script[^>]+src="assets\/(?:tauri-bridge|app)\.js"[^>]*><\/script>/g, "");
  window.document.write(markup);
  window.document.close();
  window.eval(bridgeScript);
  window.eval(appScript);

  await waitFor(
    () => window.document.querySelector("#profile-gate")?.hidden === false,
    "Ekran profili nie został pokazany"
  );
  assert.match(window.document.querySelector("#profile-list").textContent, /Mój profil/);
  assert.ok(window.document.querySelector(".profile-scene-chart"), "Brak animowanego tła logowania");
  assert.ok(window.document.querySelector("#profile-gate-settings"), "Brak ustawień wyglądu na ekranie logowania");
  assert.match(window.document.querySelector('[data-open-profile="profile-default"]').textContent, /Otwórz jednym kliknięciem/);
  window.document.querySelector('[data-open-profile="profile-default"]').click();
  await waitFor(
    () => window.document.querySelector("#app")?.hidden === false,
    "Aplikacja nie zakończyła ładowania profilu"
  );

  assert.equal(window.document.querySelectorAll(".nav-button[data-view]").length, 8);
  assert.equal(window.document.querySelector("#accounts-nav").hidden, true);
  assert.match(window.document.querySelector("#spending-limits").textContent, /Bezpiecznie dzisiaj/);
  assert.match(window.document.querySelector("#spending-limits").textContent, /Limit na 7 dni/);

  window.document.querySelector('[data-view="months"]').click();
  assert.equal(window.document.querySelectorAll("#month-tabs [data-month]").length, 12);

  window.document.querySelector('[data-view="settings"]').click();
  assert.equal(window.document.querySelector("#desktop-data-file").textContent, "portfel.sqlite");
  assert.match(window.document.querySelector("#desktop-data-path").textContent, /Roaming\\Portfel\\data/);
  assert.equal(window.document.querySelector("#desktop-app-version").textContent, "0.8.0-alpha.1");

  window.document.querySelector('[data-settings-tab="appearance"]').click();
  window.document.querySelector('input[name="login-theme"][value="ocean"]').checked = true;
  window.document.querySelector('input[name="dashboard-theme"][value="sunset"]').checked = true;
  setValue(window, "#settings-color-mode", "dark", "change");
  window.document.querySelector("#settings-motion-enabled").checked = false;
  submit(window, "#appearance-settings-form");
  assert.equal(window.document.documentElement.getAttribute("data-login-background"), "ocean");
  assert.equal(window.document.documentElement.getAttribute("data-dashboard-background"), "sunset");
  assert.equal(window.document.documentElement.getAttribute("data-motion"), "reduced");
  assert.equal(window.document.documentElement.getAttribute("data-theme"), "dark");

  window.document.querySelector('[data-settings-tab="modules"]').click();
  const bankModule = window.document.querySelector("#module-bank-accounts");
  const statementModule = window.document.querySelector("#module-statement-import");
  bankModule.checked = true;
  bankModule.dispatchEvent(new window.Event("change", { bubbles: true }));
  assert.equal(statementModule.disabled, false);
  statementModule.checked = true;
  window.document.querySelector("#save-module-settings").click();
  assert.equal(window.document.querySelector("#accounts-nav").hidden, false);

  window.document.querySelector("#accounts-nav").click();
  window.document.querySelector("#add-account").click();
  setValue(window, "#account-name", "Konto osobiste");
  setValue(window, "#account-type", "bank");
  setValue(window, "#account-opening", "2500");
  submit(window, "#account-form");
  assert.match(window.document.querySelector("#accounts-grid").textContent, /Konto osobiste/);

  window.document.querySelector('[data-view="months"]').click();
  window.document.querySelector("#add-transaction").click();
  setValue(window, "#transaction-type", "transfer", "change");
  const accountOptions = Array.from(window.document.querySelectorAll("#transaction-account option"));
  const bankOption = accountOptions.find(option => /Konto osobiste/.test(option.textContent));
  const cashOption = accountOptions.find(option => /Gotówka/.test(option.textContent));
  assert.ok(bankOption?.value, "Konto bankowe nie pojawiło się w operacji");
  assert.ok(cashOption?.value, "Gotówka nie pojawiła się w operacji");
  setValue(window, "#transaction-account", bankOption.value);
  setValue(window, "#transaction-to-account", cashOption.value);
  setValue(window, "#transaction-amount", "250");
  setValue(window, "#transaction-description", "Wypłata z bankomatu");
  submit(window, "#transaction-form");
  assert.match(window.document.querySelector("#transactions-table").textContent, /Wypłata z bankomatu/);

  window.document.querySelector('[data-view="debts"]').click();
  window.document.querySelector("#add-debt").click();
  assert.ok(window.document.querySelector("#debt-interest-enabled"));
  assert.equal(window.document.querySelector("#debt-interest-enabled").closest("label").hidden, false);
  setValue(window, "#debt-name", "Pożyczka rodzinna");
  setValue(window, "#debt-total", "3600");
  setValue(window, "#debt-repayment-mode", "flexible", "change");
  window.document.querySelector("#debt-has-deadline").checked = false;
  window.document.querySelector("#debt-has-deadline").dispatchEvent(new window.Event("change", { bubbles: true }));
  setValue(window, "#debt-minimum-payment", "300");
  submit(window, "#debt-form");
  assert.match(window.document.querySelector("#debts-grid").textContent, /Pożyczka rodzinna/);
  assert.match(window.document.querySelector("#debts-grid").textContent, /Bez rat/);
  assert.match(window.document.querySelector("#debts-grid").textContent, /bez terminu końcowego/);

  window.document.querySelector("#save-button").click();
  await waitFor(
    () => calls.some(call => call.command === "save_data"),
    "Przycisk Zapisz nie wywołał Tauri"
  );
  await waitFor(
    () => window.document.querySelector("#save-label").textContent === "Wszystko zapisane",
    "Interfejs nie potwierdził zapisu"
  );

  assert.equal(stored.version, 6);
  assert.equal(stored.settings.modules.bankAccounts, true);
  assert.equal(stored.settings.modules.statementImport, true);
  assert.equal(stored.accounts.length, 2);
  assert.equal(stored.accounts[1].name, "Konto osobiste");
  assert.equal(stored.transactions.length, 1);
  assert.equal(stored.transactions[0].type, "transfer");
  assert.equal(stored.transactions[0].accountId, bankOption.value);
  assert.equal(stored.transactions[0].toAccountId, cashOption.value);
  assert.equal(stored.settings.appearance.loginBackground, "ocean");
  assert.equal(stored.settings.appearance.dashboardBackground, "sunset");
  assert.equal(stored.settings.appearance.motionEnabled, false);
  assert.equal(stored.debts.length, 1);
  assert.equal(stored.debts[0].repaymentMode, "flexible");
  assert.equal(stored.debts[0].hasDeadline, false);
  assert.equal(stored.debts[0].hasPaymentDay, false);
  assert.equal(snapshots.length, 1);

  const preview = await window.portfelDesktop.statements.preview("bank-1", ["hash-1"]);
  assert.equal(preview.fileName, "konto.csv");
  const previewCall = calls.find(call => call.command === "preview_statement");
  assert.deepEqual(previewCall.args, {
    options: { accountId: "bank-1", existingFingerprints: ["hash-1"] }
  });

  assert.ok(calls.some(call => call.command === "list_profiles"));
  assert.ok(calls.some(call => call.command === "login_profile"));
  assert.ok(calls.some(call => call.command === "load_data"));
  assert.ok(calls.some(call => call.command === "app_info"));
  assert.ok(calls.some(call => call.command === "updater_status"));
  assert.ok(calls.some(call => call.command === "list_backups"));

  const ids = Array.from(window.document.querySelectorAll("[id]"), element => element.id);
  const duplicates = ids.filter((id, index) => ids.indexOf(id) !== index);
  assert.deepEqual(duplicates, []);

  await window.happyDOM.abort();
});

test("interfejs zawiera profesjonalne profile, motywy, długi i statusy cykliczne", async () => {
  const [tauriApp, tauriStyles, markup] = await Promise.all([
    fs.readFile(new URL("src/assets/app.js", root), "utf8"),
    fs.readFile(new URL("src/assets/styles.css", root), "utf8"),
    fs.readFile(new URL("src/index.html", root), "utf8")
  ]);
  assert.match(markup, /profile-scene-chart/);
  assert.match(markup, /profile-login-form/);
  assert.match(markup, /profile-password-toggle/);
  assert.match(markup, /data-settings-tab="appearance"/);
  assert.match(markup, /gate-login-theme/);
  assert.match(markup, /debt-repayment-mode/);
  assert.match(markup, /debt-has-deadline/);
  assert.match(markup, /debt-has-payment-day/);
  assert.match(markup, /recurring-approval-dialog/);

  assert.match(tauriApp, /APPEARANCE_THEMES/);
  assert.match(tauriApp, /openProfileLogin/);
  assert.match(tauriApp, /loginSelectedProfile/);
  assert.match(tauriApp, /repaymentMode/);
  assert.match(tauriApp, /recurringOccurrenceStatus/);
  assert.match(tauriApp, /Zatwierdzona/);
  assert.match(tauriApp, /Pominięta/);

  assert.match(tauriStyles, /profileChartDraw/);
  assert.match(tauriStyles, /profileCoinFloat/);
  assert.match(tauriStyles, /data-login-background="ocean"/);
  assert.match(tauriStyles, /data-dashboard-background="sunset"/);
  assert.match(tauriStyles, /settings-navigation/);
  assert.match(tauriStyles, /occurrence-status/);
});

test("profil chroniony używa formularza hasła i otwiera się po poprawnym haśle", async () => {
  const window = new Window({ url: "tauri://localhost/" });
  const protectedProfile = {
    id: "profile-secure", name: "Budżet domowy", color: "#7c3aed", avatarData: null,
    hasPassword: true, isDefault: true, createdAt: "2026-07-20T10:00:00.000Z", updatedAt: "2026-07-20T10:00:00.000Z"
  };
  let active = false;
  window.confirm = () => true;
  window.print = () => {};
  window.__TAURI__ = {
    core: {
      invoke: async (command, args = {}) => {
        if (command === "list_profiles") return [clone(protectedProfile)];
        if (command === "login_profile") {
          if (args.options.password !== "sekret") throw new Error("Nieprawidłowe hasło.");
          active = true;
          return clone(protectedProfile);
        }
        if (command === "load_data") {
          if (!active) throw new Error("Brak aktywnego profilu.");
          return clone(initialData);
        }
        if (command === "list_backups") return { ok: true, backups: [] };
        if (command === "app_info") return {
          version: "0.8.0-alpha.1", dataPath: "C:\\Portfel\\portfel.sqlite",
          dataDirectory: "C:\\Portfel", platform: "win32", packaged: true
        };
        if (command === "updater_status") return { state: "disabled", message: "Aktualizacje wyłączone." };
        throw new Error(`Nieobsługiwane polecenie ${command}`);
      }
    }
  };

  const markup = html
    .replace(/<link[^>]+rel="stylesheet"[^>]*>/g, "")
    .replace(/<script[^>]+src="assets\/(?:tauri-bridge|app)\.js"[^>]*><\/script>/g, "");
  window.document.write(markup);
  window.document.close();
  window.eval(bridgeScript);
  window.eval(appScript);

  await waitFor(() => window.document.querySelector("#profile-gate")?.hidden === false, "Brak ekranu profili");
  window.document.querySelector('[data-open-profile="profile-secure"]').click();
  assert.equal(window.document.querySelector("#profile-selection-view").hidden, true);
  assert.equal(window.document.querySelector("#profile-login-form").hidden, false);
  assert.equal(window.document.querySelector("#profile-login-name").textContent, "Budżet domowy");

  setValue(window, "#profile-login-password", "błędne");
  submit(window, "#profile-login-form");
  await waitFor(() => /Nieprawidłowe/.test(window.document.querySelector("#profile-login-error").textContent), "Brak błędu hasła");

  setValue(window, "#profile-login-password", "sekret");
  submit(window, "#profile-login-form");
  await waitFor(() => window.document.querySelector("#app")?.hidden === false, "Profil nie został otwarty");
  assert.equal(window.document.querySelector("#topbar-profile-name").textContent, "Budżet domowy");

  await window.happyDOM.abort();
});
