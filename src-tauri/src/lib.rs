use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Datelike, Local, SecondsFormat, Utc};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

const BACKUP_LIMIT: usize = 30;
const STATEMENT_SIZE_LIMIT: u64 = 20 * 1024 * 1024;
const DEFAULT_CATEGORIES: &[&str] = &[
    "Wypłata",
    "Dodatkowy wpływ",
    "Mieszkanie",
    "Rachunki",
    "Żywność",
    "Transport",
    "Zdrowie",
    "Ubezpieczenia",
    "Spłata długów",
    "Dom",
    "Odzież",
    "Dzieci",
    "Zwierzęta",
    "Rozrywka",
    "Rozwój",
    "Prezenty",
    "Podróże",
    "Inne",
];

#[derive(Default)]
struct StorageState {
    gate: Mutex<()>,
    active_profile: Mutex<Option<String>>,
}

struct StoragePaths {
    data_directory: PathBuf,
    database: PathBuf,
    backup_directory: PathBuf,
    legacy_database: Option<PathBuf>,
}


#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileSummary {
    id: String,
    name: String,
    color: String,
    avatar_data: Option<String>,
    has_password: bool,
    is_default: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProfileOptions {
    name: String,
    #[serde(default)]
    color: String,
    #[serde(default)]
    avatar_data: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    is_default: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginProfileOptions {
    profile_id: String,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProfileOptions {
    name: String,
    #[serde(default)]
    color: String,
    #[serde(default)]
    avatar_data: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    remove_password: bool,
    #[serde(default)]
    is_default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    ok: bool,
    saved_at: String,
    path: String,
    backup: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupInfo {
    name: String,
    size: u64,
    created_at: String,
}

#[derive(Serialize)]
struct BackupListResult {
    ok: bool,
    backups: Vec<BackupInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreResult {
    ok: bool,
    restored: String,
    backup: Option<String>,
    saved_at: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    data_path: String,
    data_directory: String,
    platform: &'static str,
    packaged: bool,
}

#[derive(Serialize)]
struct UpdateStatus {
    state: &'static str,
    message: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatementOptions {
    account_id: String,
    #[serde(default)]
    existing_fingerprints: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatementRow {
    row_number: usize,
    date: String,
    amount: f64,
    description: String,
    external_id: String,
    fingerprint: String,
    category: String,
    duplicate: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatementPreview {
    ok: bool,
    file_name: String,
    delimiter: String,
    headers: Vec<String>,
    rows: Vec<StatementRow>,
    skipped: usize,
    warnings: Vec<String>,
}

#[derive(Default)]
struct HeaderMapping {
    date: Option<usize>,
    amount: Option<usize>,
    debit: Option<usize>,
    credit: Option<usize>,
    description: Option<usize>,
    external_id: Option<usize>,
    category: Option<usize>,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn storage_paths(app: &AppHandle) -> Result<StoragePaths, String> {
    let generated = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Nie udało się ustalić katalogu danych: {error}"))?;
    let canonical_root = generated
        .parent()
        .map(|parent| parent.join("Portfel"))
        .unwrap_or_else(|| generated.clone());
    let data_directory = canonical_root.join("data");
    let legacy_database = generated
        .ne(&canonical_root)
        .then(|| generated.join("data").join("portfel.sqlite"));

    Ok(StoragePaths {
        database: data_directory.join("portfel.sqlite"),
        backup_directory: data_directory.join("backups"),
        data_directory,
        legacy_database,
    })
}

fn empty_data() -> Value {
    let year = Local::now().year();
    let created_at = now_iso();
    json!({
        "version": 5,
        "meta": { "savedAt": null, "createdAt": created_at },
        "settings": {
            "currentYear": year,
            "theme": "light",
            "currency": "PLN",
            "balances": { year.to_string(): { "available": 0, "reserve": 0 } },
            "categories": DEFAULT_CATEGORIES,
            "modules": {
                "bankAccounts": false,
                "statementImport": false,
                "alerts": true,
                "dailyLimit": true,
                "weeklyLimit": true,
                "interest": true,
                "recurring": true,
                "categoryBudgets": true
            },
            "payday": { "day": 10, "nextDate": "" }
        },
        "accounts": [{
            "id": "cash-main",
            "name": "Gotówka",
            "type": "cash",
            "currency": "PLN",
            "openingBalances": { year.to_string(): 0 },
            "active": true,
            "includeInSpendingLimit": true,
            "createdAt": created_at
        }],
        "goals": [],
        "debts": [],
        "recurring": [],
        "budgets": [],
        "transactions": []
    })
}

fn numeric_year(value: Option<&Value>) -> i32 {
    let parsed = value
        .and_then(|item| {
            item.as_i64()
                .or_else(|| item.as_u64().map(|number| number as i64))
                .or_else(|| item.as_str().and_then(|text| text.parse::<i64>().ok()))
        })
        .unwrap_or_else(|| Local::now().year() as i64);
    parsed.clamp(2020, 2100) as i32
}

fn number_value(value: Option<&Value>) -> f64 {
    value
        .and_then(|item| {
            item.as_f64()
                .or_else(|| item.as_str().and_then(|text| text.parse::<f64>().ok()))
        })
        .filter(|number| number.is_finite())
        .unwrap_or(0.0)
}

fn merge_defaults(target: &mut Map<String, Value>, defaults: &Map<String, Value>) {
    for (key, value) in defaults {
        target.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

fn migrate_payload(mut payload: Value) -> Result<Value, String> {
    let defaults = empty_data();
    let defaults_settings = defaults["settings"]
        .as_object()
        .expect("domyślne ustawienia są obiektem");
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "Główny element danych musi być obiektem.".to_string())?;

    let opening_available = {
        let settings_value = object
            .entry("settings".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !settings_value.is_object() {
            *settings_value = Value::Object(Map::new());
        }
        let settings = settings_value
            .as_object_mut()
            .expect("ustawienia zostały ustawione jako obiekt");
        merge_defaults(settings, defaults_settings);

        let year = numeric_year(settings.get("currentYear"));
        settings.insert("currentYear".to_string(), json!(year));
        settings.insert("currency".to_string(), json!("PLN"));
        let theme = if settings.get("theme").and_then(Value::as_str) == Some("dark") {
            "dark"
        } else {
            "light"
        };
        settings.insert("theme".to_string(), json!(theme));

        let valid_categories = settings
            .get("categories")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty());
        if !valid_categories {
            settings.insert("categories".to_string(), json!(DEFAULT_CATEGORIES));
        }

        for (key, default_value) in [
            ("modules", &defaults_settings["modules"]),
            ("payday", &defaults_settings["payday"]),
        ] {
            let target = settings
                .entry(key.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !target.is_object() {
                *target = Value::Object(Map::new());
            }
            merge_defaults(
                target
                    .as_object_mut()
                    .expect("ustawienie zostało ustawione jako obiekt"),
                default_value
                    .as_object()
                    .expect("domyślne ustawienie jest obiektem"),
            );
        }

        let balances = settings
            .entry("balances".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !balances.is_object() {
            *balances = Value::Object(Map::new());
        }
        let balances = balances
            .as_object_mut()
            .expect("salda zostały ustawione jako obiekt");
        let balance = balances
            .entry(year.to_string())
            .or_insert_with(|| json!({ "available": 0, "reserve": 0 }));
        if !balance.is_object() {
            *balance = json!({ "available": 0, "reserve": 0 });
        }
        number_value(balance.get("available")).max(0.0)
    };

    let needs_cash_account = object
        .get("accounts")
        .and_then(Value::as_array)
        .is_none_or(|accounts| accounts.is_empty());
    if needs_cash_account {
        let year = object["settings"]["currentYear"]
            .as_i64()
            .unwrap_or_else(|| Local::now().year() as i64);
        object.insert(
            "accounts".to_string(),
            json!([{
                "id": "cash-main",
                "name": "Gotówka",
                "type": "cash",
                "currency": "PLN",
                "openingBalances": { year.to_string(): opening_available },
                "active": true,
                "includeInSpendingLimit": true,
                "createdAt": now_iso()
            }]),
        );
    }

    for key in [
        "accounts",
        "goals",
        "debts",
        "recurring",
        "budgets",
        "transactions",
    ] {
        let list = object
            .entry(key.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let list = list
            .as_array()
            .ok_or_else(|| format!("Pole „{key}” musi być listą."))?;
        if list.len() > 100_000 {
            return Err(format!("Pole „{key}” zawiera zbyt wiele elementów."));
        }
    }

    let meta = object
        .entry("meta".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !meta.is_object() {
        *meta = Value::Object(Map::new());
    }
    meta.as_object_mut()
        .expect("metadane zostały ustawione jako obiekt")
        .entry("createdAt".to_string())
        .or_insert_with(|| Value::String(now_iso()));
    object.insert("version".to_string(), json!(5));
    Ok(payload)
}

fn validate_for_save(payload: Value) -> Result<Value, String> {
    let mut payload = migrate_payload(payload)?;
    payload["meta"]["savedAt"] = Value::String(now_iso());
    Ok(payload)
}

fn serialize_payload(payload: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(payload)
        .map(|serialized| format!("{serialized}\n"))
        .map_err(|error| format!("Nie udało się przygotować danych do zapisu: {error}"))
}

fn read_state_from_database(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let serialized: String = connection
        .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| row.get(0))
        .ok()?;
    serde_json::from_str(&serialized).ok()
}

fn normalized_profile_name(value: &str) -> Result<String, String> {
    let name = value.trim();
    if name.is_empty() {
        return Err("Nazwa profilu nie może być pusta.".to_string());
    }
    if name.chars().count() > 80 {
        return Err("Nazwa profilu może mieć maksymalnie 80 znaków.".to_string());
    }
    Ok(name.to_string())
}

fn normalized_profile_color(value: &str) -> String {
    let value = value.trim();
    if value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|character| character.is_ascii_hexdigit())
    {
        value.to_ascii_lowercase()
    } else {
        "#2563eb".to_string()
    }
}

fn normalized_avatar(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 2_500_000 {
        return Err("Zdjęcie profilu jest zbyt duże.".to_string());
    }
    if !value.starts_with("data:image/") {
        return Err("Nieprawidłowy format zdjęcia profilu.".to_string());
    }
    Ok(Some(value))
}

fn hash_password(password: &str) -> Result<Option<String>, String> {
    if password.is_empty() {
        return Ok(None);
    }
    if password.chars().count() > 256 {
        return Err("Hasło jest zbyt długie.".to_string());
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| Some(hash.to_string()))
        .map_err(|error| format!("Nie udało się zabezpieczyć hasła: {error}"))
}

fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

fn open_database(paths: &StoragePaths) -> Result<Connection, String> {
    fs::create_dir_all(&paths.data_directory)
        .map_err(|error| format!("Nie udało się utworzyć katalogu danych: {error}"))?;
    fs::create_dir_all(&paths.backup_directory)
        .map_err(|error| format!("Nie udało się utworzyć katalogu kopii: {error}"))?;

    let connection = Connection::open(&paths.database)
        .map_err(|error| format!("Nie udało się otworzyć lokalnej bazy: {error}"))?;
    connection
        .execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS app_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                schema_version INTEGER NOT NULL,
                payload TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                avatar_data TEXT,
                password_hash TEXT,
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS one_default_profile
            ON profiles(is_default) WHERE is_default = 1;

            CREATE TABLE IF NOT EXISTS profile_state (
                profile_id TEXT PRIMARY KEY,
                schema_version INTEGER NOT NULL,
                payload TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
            );
            "#,
        )
        .map_err(|error| format!("Nie udało się przygotować lokalnej bazy: {error}"))?;

    let has_state: bool = connection
        .query_row("SELECT EXISTS(SELECT 1 FROM app_state WHERE id = 1)", [], |row| row.get(0))
        .map_err(|error| format!("Nie udało się sprawdzić lokalnych danych: {error}"))?;
    if !has_state {
        let initial = paths
            .legacy_database
            .as_deref()
            .and_then(read_state_from_database)
            .unwrap_or_else(empty_data);
        let initial = migrate_payload(initial)?;
        let serialized = serialize_payload(&initial)?;
        let updated_at = initial["meta"]["savedAt"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(now_iso);
        connection
            .execute(
                "INSERT INTO app_state(id, schema_version, payload, updated_at) VALUES(1, 5, ?1, ?2)",
                params![serialized, updated_at],
            )
            .map_err(|error| format!("Nie udało się utworzyć pierwszego zapisu: {error}"))?;
    }

    let has_profiles: bool = connection
        .query_row("SELECT EXISTS(SELECT 1 FROM profiles)", [], |row| row.get(0))
        .map_err(|error| format!("Nie udało się sprawdzić profili: {error}"))?;
    if !has_profiles {
        let serialized: String = connection
            .query_row("SELECT payload FROM app_state WHERE id = 1", [], |row| row.get(0))
            .map_err(|error| format!("Nie udało się przenieść dotychczasowych danych: {error}"))?;
        let now = now_iso();
        connection
            .execute(
                "INSERT INTO profiles(id, name, color, avatar_data, password_hash, is_default, created_at, updated_at) VALUES('profile-default', 'Mój profil', '#2563eb', NULL, NULL, 1, ?1, ?1)",
                params![now],
            )
            .map_err(|error| format!("Nie udało się utworzyć profilu domyślnego: {error}"))?;
        connection
            .execute(
                "INSERT INTO profile_state(profile_id, schema_version, payload, updated_at) VALUES('profile-default', 5, ?1, ?2)",
                params![serialized, now],
            )
            .map_err(|error| format!("Nie udało się przypisać danych do profilu: {error}"))?;
    }
    Ok(connection)
}

fn profile_summary(connection: &Connection, profile_id: &str) -> Result<ProfileSummary, String> {
    connection
        .query_row(
            r#"
            SELECT id, name, color, avatar_data,
                   CASE WHEN password_hash IS NULL OR password_hash = '' THEN 0 ELSE 1 END,
                   is_default, created_at, updated_at
            FROM profiles WHERE id = ?1
            "#,
            params![profile_id],
            |row| {
                Ok(ProfileSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    avatar_data: row.get(3)?,
                    has_password: row.get::<_, i64>(4)? != 0,
                    is_default: row.get::<_, i64>(5)? != 0,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .map_err(|_| "Nie znaleziono profilu.".to_string())
}

fn require_active_profile(state: &State<'_, StorageState>) -> Result<String, String> {
    state
        .active_profile
        .lock()
        .map_err(|_| "Sesja profilu jest chwilowo niedostępna.".to_string())?
        .clone()
        .ok_or_else(|| "Najpierw wybierz profil użytkownika.".to_string())
}

fn current_serialized(connection: &Connection, profile_id: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT payload FROM profile_state WHERE profile_id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("Nie udało się odczytać lokalnych danych: {error}"))
}

fn profile_backup_directory(paths: &StoragePaths, profile_id: &str) -> PathBuf {
    paths.backup_directory.join(profile_id)
}

fn backup_file_paths(paths: &StoragePaths, profile_id: &str) -> Result<Vec<PathBuf>, String> {
    let directory = profile_backup_directory(paths, profile_id);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Nie udało się utworzyć katalogu kopii: {error}"))?;
    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("Nie udało się odczytać katalogu kopii: {error}"))?;
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("portfel-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    Ok(files)
}

fn create_backup(paths: &StoragePaths, profile_id: &str, serialized: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(serialized)
        .map_err(|error| format!("Nie udało się przygotować kopii danych: {error}"))?;
    let pretty = serialize_payload(&parsed)?;
    let directory = profile_backup_directory(paths, profile_id);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Nie udało się utworzyć katalogu kopii: {error}"))?;
    let base = Local::now();
    let (name, destination) = (0..1000)
        .map(|offset| base + chrono::Duration::milliseconds(offset))
        .map(|date| {
            let name = format!("portfel-{}.json", date.format("%Y%m%d-%H%M%S-%3f"));
            let path = directory.join(&name);
            (name, path)
        })
        .find(|(_, path)| !path.exists())
        .ok_or_else(|| "Nie udało się wybrać nazwy kopii danych.".to_string())?;
    let temporary = directory.join(format!("{name}.tmp"));
    fs::write(&temporary, pretty)
        .map_err(|error| format!("Nie udało się zapisać kopii danych: {error}"))?;
    fs::rename(&temporary, &destination)
        .map_err(|error| format!("Nie udało się zatwierdzić kopii danych: {error}"))?;

    for old in backup_file_paths(paths, profile_id)?.into_iter().skip(BACKUP_LIMIT) {
        fs::remove_file(old)
            .map_err(|error| format!("Nie udało się usunąć starej kopii danych: {error}"))?;
    }
    Ok(name)
}

fn write_state(
    connection: &mut Connection,
    profile_id: &str,
    payload: &Value,
) -> Result<(), String> {
    let serialized = serialize_payload(payload)?;
    let saved_at = payload["meta"]["savedAt"].as_str().unwrap_or_default();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Nie udało się rozpocząć zapisu: {error}"))?;
    transaction
        .execute(
            r#"
            INSERT INTO profile_state(profile_id, schema_version, payload, updated_at)
            VALUES(?1, 5, ?2, ?3)
            ON CONFLICT(profile_id) DO UPDATE SET
                schema_version = 5,
                payload = excluded.payload,
                updated_at = excluded.updated_at
            "#,
            params![profile_id, serialized, saved_at],
        )
        .map_err(|error| format!("Nie udało się zapisać danych: {error}"))?;
    transaction
        .execute(
            "UPDATE profiles SET updated_at = ?2 WHERE id = ?1",
            params![profile_id, saved_at],
        )
        .map_err(|error| format!("Nie udało się zaktualizować profilu: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Nie udało się zakończyć zapisu: {error}"))
}

#[tauri::command]
fn list_profiles(app: AppHandle, state: State<'_, StorageState>) -> Result<Vec<ProfileSummary>, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let paths = storage_paths(&app)?;
    let connection = open_database(&paths)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name, color, avatar_data,
                   CASE WHEN password_hash IS NULL OR password_hash = '' THEN 0 ELSE 1 END,
                   is_default, created_at, updated_at
            FROM profiles
            ORDER BY is_default DESC, updated_at DESC, name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("Nie udało się przygotować listy profili: {error}"))?;
    let profiles = statement
        .query_map([], |row| {
            Ok(ProfileSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                avatar_data: row.get(3)?,
                has_password: row.get::<_, i64>(4)? != 0,
                is_default: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|error| format!("Nie udało się odczytać profili: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Nie udało się odczytać profili: {error}"))?;
    Ok(profiles)
}

#[tauri::command]
fn create_profile(
    options: CreateProfileOptions,
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<ProfileSummary, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let paths = storage_paths(&app)?;
    let mut connection = open_database(&paths)?;
    let name = normalized_profile_name(&options.name)?;
    let color = normalized_profile_color(&options.color);
    let avatar = normalized_avatar(options.avatar_data)?;
    let password_hash = hash_password(options.password.as_deref().unwrap_or_default())?;
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
        .map_err(|error| format!("Nie udało się policzyć profili: {error}"))?;
    let is_default = options.is_default || count == 0;
    let mut profile_id = format!("profile-{}", Local::now().format("%Y%m%d%H%M%S%3f"));
    let mut suffix = 0;
    while connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
            params![profile_id],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false)
    {
        suffix += 1;
        profile_id = format!("profile-{}-{suffix}", Local::now().format("%Y%m%d%H%M%S%3f"));
    }
    let now = now_iso();
    let initial = migrate_payload(empty_data())?;
    let serialized = serialize_payload(&initial)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Nie udało się rozpocząć tworzenia profilu: {error}"))?;
    if is_default {
        transaction
            .execute("UPDATE profiles SET is_default = 0", [])
            .map_err(|error| format!("Nie udało się zmienić profilu domyślnego: {error}"))?;
    }
    transaction
        .execute(
            "INSERT INTO profiles(id, name, color, avatar_data, password_hash, is_default, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![profile_id, name, color, avatar, password_hash, is_default as i64, now],
        )
        .map_err(|error| format!("Nie udało się utworzyć profilu: {error}"))?;
    transaction
        .execute(
            "INSERT INTO profile_state(profile_id, schema_version, payload, updated_at) VALUES(?1, 5, ?2, ?3)",
            params![profile_id, serialized, now],
        )
        .map_err(|error| format!("Nie udało się utworzyć danych profilu: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Nie udało się zakończyć tworzenia profilu: {error}"))?;
    *state
        .active_profile
        .lock()
        .map_err(|_| "Sesja profilu jest chwilowo niedostępna.".to_string())? = Some(profile_id.clone());
    profile_summary(&connection, &profile_id)
}

#[tauri::command]
fn login_profile(
    options: LoginProfileOptions,
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<ProfileSummary, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let paths = storage_paths(&app)?;
    let connection = open_database(&paths)?;
    let password_hash: Option<String> = connection
        .query_row(
            "SELECT password_hash FROM profiles WHERE id = ?1",
            params![options.profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("Nie udało się odczytać profilu: {error}"))?
        .ok_or_else(|| "Nie znaleziono profilu.".to_string())?;
    if let Some(encoded) = password_hash.filter(|value| !value.is_empty()) {
        let provided = options.password.as_deref().unwrap_or_default();
        if !verify_password(provided, &encoded) {
            return Err("Nieprawidłowe hasło.".to_string());
        }
    }
    *state
        .active_profile
        .lock()
        .map_err(|_| "Sesja profilu jest chwilowo niedostępna.".to_string())? = Some(options.profile_id.clone());
    profile_summary(&connection, &options.profile_id)
}

#[tauri::command]
fn logout_profile(state: State<'_, StorageState>) -> Result<bool, String> {
    *state
        .active_profile
        .lock()
        .map_err(|_| "Sesja profilu jest chwilowo niedostępna.".to_string())? = None;
    Ok(true)
}

#[tauri::command]
fn update_profile(
    options: UpdateProfileOptions,
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<ProfileSummary, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let profile_id = require_active_profile(&state)?;
    let paths = storage_paths(&app)?;
    let mut connection = open_database(&paths)?;
    let name = normalized_profile_name(&options.name)?;
    let color = normalized_profile_color(&options.color);
    let avatar = normalized_avatar(options.avatar_data)?;
    let existing_password: Option<String> = connection
        .query_row(
            "SELECT password_hash FROM profiles WHERE id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Nie udało się odczytać profilu: {error}"))?;
    let password_hash = if options.remove_password {
        None
    } else if let Some(password) = options.password.filter(|value| !value.is_empty()) {
        hash_password(&password)?
    } else {
        existing_password
    };
    let now = now_iso();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Nie udało się rozpocząć aktualizacji profilu: {error}"))?;
    if options.is_default {
        transaction
            .execute("UPDATE profiles SET is_default = 0", [])
            .map_err(|error| format!("Nie udało się zmienić profilu domyślnego: {error}"))?;
    }
    transaction
        .execute(
            "UPDATE profiles SET name = ?2, color = ?3, avatar_data = ?4, password_hash = ?5, is_default = CASE WHEN ?6 = 1 THEN 1 ELSE is_default END, updated_at = ?7 WHERE id = ?1",
            params![profile_id, name, color, avatar, password_hash, options.is_default as i64, now],
        )
        .map_err(|error| format!("Nie udało się zaktualizować profilu: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Nie udało się zakończyć aktualizacji profilu: {error}"))?;
    profile_summary(&connection, &profile_id)
}

#[tauri::command]
fn load_data(app: AppHandle, state: State<'_, StorageState>) -> Result<Value, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let profile_id = require_active_profile(&state)?;
    let paths = storage_paths(&app)?;
    let connection = open_database(&paths)?;
    let serialized = current_serialized(&connection, &profile_id)?
        .ok_or_else(|| "Nie znaleziono danych profilu.".to_string())?;
    let parsed: Value = serde_json::from_str(&serialized)
        .map_err(|error| format!("Zapisane dane są uszkodzone: {error}"))?;
    migrate_payload(parsed)
}

#[tauri::command]
fn save_data(
    payload: Value,
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<SaveResult, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let profile_id = require_active_profile(&state)?;
    let paths = storage_paths(&app)?;
    let mut connection = open_database(&paths)?;
    let payload = validate_for_save(payload)?;
    let backup = current_serialized(&connection, &profile_id)?
        .map(|current| create_backup(&paths, &profile_id, &current))
        .transpose()?;
    write_state(&mut connection, &profile_id, &payload)?;
    let saved_at = payload["meta"]["savedAt"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(SaveResult {
        ok: true,
        saved_at,
        path: paths.database.to_string_lossy().into_owned(),
        backup,
    })
}

#[tauri::command]
fn list_backups(
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<BackupListResult, String> {
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let profile_id = require_active_profile(&state)?;
    let paths = storage_paths(&app)?;
    let backups = backup_file_paths(&paths, &profile_id)?
        .into_iter()
        .filter_map(|path| {
            let metadata = fs::metadata(&path).ok()?;
            let modified: DateTime<Utc> = metadata.modified().ok()?.into();
            Some(BackupInfo {
                name: path.file_name()?.to_string_lossy().into_owned(),
                size: metadata.len(),
                created_at: modified.to_rfc3339_opts(SecondsFormat::Millis, true),
            })
        })
        .collect();
    Ok(BackupListResult { ok: true, backups })
}

fn valid_backup_name(name: &str) -> bool {
    !name.contains('/')
        && !name.contains('\\')
        && name.starts_with("portfel-")
        && name.ends_with(".json")
}

#[tauri::command]
fn restore_backup(
    name: String,
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<RestoreResult, String> {
    if !valid_backup_name(&name) {
        return Err("Nieprawidłowa nazwa kopii danych.".to_string());
    }
    let _guard = state
        .gate
        .lock()
        .map_err(|_| "Magazyn danych jest chwilowo niedostępny.".to_string())?;
    let profile_id = require_active_profile(&state)?;
    let paths = storage_paths(&app)?;
    let selected_path = profile_backup_directory(&paths, &profile_id).join(&name);
    if !selected_path.is_file() {
        return Err("Nie znaleziono wybranej kopii danych.".to_string());
    }
    let selected = fs::read_to_string(&selected_path)
        .map_err(|error| format!("Nie udało się odczytać wybranej kopii: {error}"))?;
    let selected: Value = serde_json::from_str(&selected)
        .map_err(|error| format!("Wybrana kopia jest uszkodzona: {error}"))?;
    let selected = validate_for_save(selected)?;

    let mut connection = open_database(&paths)?;
    let backup = current_serialized(&connection, &profile_id)?
        .map(|current| create_backup(&paths, &profile_id, &current))
        .transpose()?;
    write_state(&mut connection, &profile_id, &selected)?;
    let saved_at = selected["meta"]["savedAt"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(RestoreResult {
        ok: true,
        restored: name,
        backup,
        saved_at,
        path: paths.database.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
fn export_data(payload: Value) -> Result<Value, String> {
    let selected = rfd::FileDialog::new()
        .set_title("Eksportuj kopię danych")
        .set_file_name(format!(
            "portfel-kopia-{}.json",
            Local::now().format("%Y-%m-%d")
        ))
        .add_filter("Kopia Portfela", &["json"])
        .save_file();
    let Some(path) = selected else {
        return Ok(json!({ "ok": false, "canceled": true }));
    };
    let serialized = serialize_payload(&payload)?;
    fs::write(&path, serialized)
        .map_err(|error| format!("Nie udało się wyeksportować kopii: {error}"))?;
    Ok(json!({ "ok": true, "path": path.to_string_lossy() }))
}

#[tauri::command]
fn import_data() -> Result<Value, String> {
    let selected = rfd::FileDialog::new()
        .set_title("Importuj kopię danych")
        .add_filter("Kopia Portfela", &["json"])
        .pick_file();
    let Some(path) = selected else {
        return Ok(json!({ "ok": false, "canceled": true }));
    };
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Nie udało się odczytać kopii: {error}"))?;
    if metadata.len() > 50 * 1024 * 1024 {
        return Err("Wybrana kopia jest zbyt duża.".to_string());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Nie udało się odczytać kopii: {error}"))?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|error| format!("Wybrany plik nie jest prawidłową kopią JSON: {error}"))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(json!({ "ok": true, "data": parsed, "fileName": file_name }))
}

fn normalized(value: &str) -> String {
    let mut output = String::new();
    let mut separated = false;
    for character in value.nfd().filter(|character| !is_combining_mark(*character)) {
        for lower in character.to_lowercase() {
            if lower.is_ascii_alphanumeric() {
                output.push(lower);
                separated = false;
            } else if !output.is_empty() && !separated {
                output.push(' ');
                separated = true;
            }
        }
    }
    output.trim().to_string()
}

fn detect_delimiter(first_line: &str) -> char {
    let mut selected = ';';
    let mut selected_count = first_line.matches(';').count();
    for candidate in [',', '\t'] {
        let count = first_line.matches(candidate).count();
        if count > selected_count {
            selected = candidate;
            selected_count = count;
        }
    }
    selected
}

fn parse_csv(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '"' {
            if quoted && characters.get(index + 1) == Some(&'"') {
                value.push('"');
                index += 1;
            } else {
                quoted = !quoted;
            }
        } else if character == delimiter && !quoted {
            row.push(value.trim().to_string());
            value.clear();
        } else if (character == '\n' || character == '\r') && !quoted {
            if character == '\r' && characters.get(index + 1) == Some(&'\n') {
                index += 1;
            }
            row.push(value.trim().to_string());
            value.clear();
            if row.iter().any(|cell| !cell.is_empty()) {
                rows.push(row);
            }
            row = Vec::new();
        } else {
            value.push(character);
        }
        index += 1;
    }
    if !value.is_empty() || !row.is_empty() {
        row.push(value.trim().to_string());
        if row.iter().any(|cell| !cell.is_empty()) {
            rows.push(row);
        }
    }
    rows
}

fn select_header(headers: &[String], candidates: &[&str]) -> Option<usize> {
    let candidates = candidates
        .iter()
        .map(|candidate| normalized(candidate))
        .collect::<Vec<_>>();
    headers.iter().position(|header| {
        let key = normalized(header);
        candidates
            .iter()
            .any(|candidate| key == *candidate || key.contains(candidate))
    })
}

fn detect_mapping(headers: &[String]) -> HeaderMapping {
    HeaderMapping {
        date: select_header(
            headers,
            &[
                "data operacji",
                "data transakcji",
                "data księgowania",
                "booking date",
                "transaction date",
                "date",
            ],
        ),
        amount: select_header(
            headers,
            &["kwota", "kwota operacji", "kwota transakcji", "amount", "wartość"],
        ),
        debit: select_header(
            headers,
            &["obciążenie", "kwota obciążenia", "debit", "winien"],
        ),
        credit: select_header(
            headers,
            &["uznanie", "kwota uznania", "credit", "ma"],
        ),
        description: select_header(
            headers,
            &[
                "opis transakcji",
                "opis operacji",
                "tytuł",
                "szczegóły",
                "description",
                "nazwa kontrahenta",
            ],
        ),
        external_id: select_header(
            headers,
            &[
                "identyfikator",
                "id transakcji",
                "numer referencyjny",
                "reference",
                "transaction id",
            ],
        ),
        category: select_header(headers, &["kategoria", "category"]),
    }
}

fn parse_amount(value: &str) -> Option<f64> {
    let mut cleaned = value
        .replace('\u{00a0}', " ")
        .chars()
        .filter(|character| {
            !character.is_whitespace()
                && !character.is_ascii_alphabetic()
                && !matches!(character, 'ł' | 'Ł' | '€' | '$')
        })
        .collect::<String>();
    if cleaned.is_empty() {
        return None;
    }
    let negative = (cleaned.starts_with('(') && cleaned.ends_with(')')) || cleaned.starts_with('-');
    cleaned = cleaned.replace(['(', ')'], "");
    if cleaned.starts_with(['-', '+']) {
        cleaned.remove(0);
    }
    if cleaned.contains(',') && cleaned.contains('.') {
        if cleaned.rfind(',') > cleaned.rfind('.') {
            cleaned = cleaned.replace('.', "").replacen(',', ".", 1);
        } else {
            cleaned = cleaned.replace(',', "");
        }
    } else if cleaned.contains(',') {
        cleaned = cleaned.replace('.', "").replacen(',', ".", 1);
    }
    let parsed = cleaned.parse::<f64>().ok()?;
    if !parsed.is_finite() {
        return None;
    }
    Some(if negative { -parsed.abs() } else { parsed })
}

fn parse_date(value: &str) -> Option<String> {
    let prefix = value
        .trim()
        .chars()
        .take_while(|character| character.is_ascii_digit() || matches!(character, '-' | '/' | '.'))
        .collect::<String>();
    let parts = prefix
        .split(['-', '/', '.'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    if parts[0].len() == 4 {
        return Some(format!("{}-{:0>2}-{:0>2}", parts[0], parts[1], parts[2]));
    }
    if parts[2].len() == 4 {
        return Some(format!("{}-{:0>2}-{:0>2}", parts[2], parts[1], parts[0]));
    }
    None
}

fn statement_fingerprint(
    account_id: &str,
    date: &str,
    amount: f64,
    description: &str,
    external_id: &str,
) -> String {
    let source = format!(
        "{account_id}|{date}|{amount:.2}|{}|{external_id}",
        normalized(description)
    );
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

fn parse_statement_text(
    text: &str,
    file_name: &str,
    account_id: &str,
    existing_fingerprints: &[String],
) -> Result<StatementPreview, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first_line = text.lines().next().unwrap_or_default();
    let delimiter = detect_delimiter(first_line);
    let parsed = parse_csv(text, delimiter);
    if parsed.len() < 2 {
        return Err("Wyciąg nie zawiera operacji.".to_string());
    }
    let headers = parsed[0]
        .iter()
        .enumerate()
        .map(|(index, header)| {
            if header.is_empty() {
                format!("Kolumna {}", index + 1)
            } else {
                header.clone()
            }
        })
        .collect::<Vec<_>>();
    let mapping = detect_mapping(&headers);
    if mapping.date.is_none()
        || (mapping.amount.is_none() && mapping.debit.is_none() && mapping.credit.is_none())
    {
        return Err(
            "Nie rozpoznano kolumny daty lub kwoty. Wyeksportuj wyciąg CSV z nagłówkami."
                .to_string(),
        );
    }
    let existing = existing_fingerprints.iter().collect::<HashSet<_>>();
    let mut rows = Vec::new();
    let mut skipped = 0;
    for (index, source) in parsed.iter().enumerate().skip(1) {
        let cell = |column: Option<usize>| -> &str {
            column
                .and_then(|column| source.get(column))
                .map(String::as_str)
                .unwrap_or_default()
                .trim()
        };
        let date = mapping.date.and_then(|column| parse_date(cell(Some(column))));
        let mut amount = mapping.amount.and_then(|column| parse_amount(cell(Some(column))));
        if amount.is_none() && (mapping.debit.is_some() || mapping.credit.is_some()) {
            let debit = parse_amount(cell(mapping.debit)).unwrap_or(0.0);
            let credit = parse_amount(cell(mapping.credit)).unwrap_or(0.0);
            amount = Some(if credit != 0.0 {
                credit.abs()
            } else {
                -debit.abs()
            });
        }
        let Some(date) = date else {
            skipped += 1;
            continue;
        };
        let Some(amount) = amount else {
            skipped += 1;
            continue;
        };
        if amount == 0.0 {
            skipped += 1;
            continue;
        }
        let description = match cell(mapping.description) {
            "" => "Operacja bankowa".to_string(),
            value => value.to_string(),
        };
        let external_id = cell(mapping.external_id).to_string();
        let fingerprint = statement_fingerprint(
            account_id,
            &date,
            amount,
            &description,
            &external_id,
        );
        rows.push(StatementRow {
            row_number: index + 1,
            date,
            amount,
            description,
            external_id,
            duplicate: existing.contains(&fingerprint),
            fingerprint,
            category: cell(mapping.category).to_string(),
        });
    }
    if rows.is_empty() {
        return Err("Nie znaleziono prawidłowych operacji w wyciągu.".to_string());
    }
    let warnings = if skipped > 0 {
        vec![format!(
            "Pominięto {skipped} wierszy bez prawidłowej daty lub kwoty."
        )]
    } else {
        Vec::new()
    };
    Ok(StatementPreview {
        ok: true,
        file_name: file_name.to_string(),
        delimiter: if delimiter == '\t' {
            "tabulator".to_string()
        } else {
            delimiter.to_string()
        },
        headers,
        rows,
        skipped,
        warnings,
    })
}

fn preview_statement_file(
    path: &Path,
    account_id: &str,
    existing_fingerprints: &[String],
) -> Result<StatementPreview, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if extension != "csv" && extension != "txt" {
        return Err(
            "Na tym etapie obsługiwane są wyciągi CSV lub TXT wyeksportowane z banku."
                .to_string(),
        );
    }
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Nie udało się odczytać wyciągu: {error}"))?;
    if metadata.len() > STATEMENT_SIZE_LIMIT {
        return Err("Plik jest zbyt duży.".to_string());
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("Nie udało się odczytać wyciągu: {error}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| "Wyciąg musi być zapisany jako plik tekstowy UTF-8.".to_string())?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "wyciąg.csv".to_string());
    parse_statement_text(&text, &file_name, account_id, existing_fingerprints)
}

#[tauri::command]
fn preview_statement(options: StatementOptions) -> Result<Value, String> {
    if options.account_id.trim().is_empty() {
        return Err("Najpierw wybierz konto bankowe.".to_string());
    }
    let selected = rfd::FileDialog::new()
        .set_title("Wybierz wyciąg bankowy")
        .add_filter("Wyciąg CSV", &["csv", "txt"])
        .pick_file();
    let Some(path) = selected else {
        return Ok(json!({ "ok": false, "canceled": true }));
    };
    let fingerprints = options
        .existing_fingerprints
        .into_iter()
        .take(100_000)
        .collect::<Vec<_>>();
    serde_json::to_value(preview_statement_file(
        &path,
        options.account_id.trim(),
        &fingerprints,
    )?)
    .map_err(|error| format!("Nie udało się przygotować podglądu wyciągu: {error}"))
}

#[tauri::command]
fn app_info(app: AppHandle) -> Result<AppInfo, String> {
    let paths = storage_paths(&app)?;
    let platform = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        _ => std::env::consts::OS,
    };
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_path: paths.database.to_string_lossy().into_owned(),
        data_directory: paths.data_directory.to_string_lossy().into_owned(),
        platform,
        packaged: true,
    })
}

fn open_directory(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = Command::new("explorer.exe").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(path).spawn();
    result
        .map(|_| ())
        .map_err(|error| format!("Nie udało się otworzyć folderu: {error}"))
}

#[tauri::command]
fn open_data_folder(app: AppHandle) -> Result<String, String> {
    let paths = storage_paths(&app)?;
    fs::create_dir_all(&paths.data_directory)
        .map_err(|error| format!("Nie udało się utworzyć katalogu danych: {error}"))?;
    Ok(match open_directory(&paths.data_directory) {
        Ok(()) => String::new(),
        Err(message) => message,
    })
}

fn disabled_update_status() -> UpdateStatus {
    UpdateStatus {
        state: "disabled",
        message: "Kanał aktualizacji zostanie aktywowany przy publikacji programu.",
    }
}

#[tauri::command]
fn updater_status() -> UpdateStatus {
    disabled_update_status()
}

#[tauri::command]
fn updater_check() -> UpdateStatus {
    disabled_update_status()
}

#[tauri::command]
fn updater_install() -> UpdateStatus {
    disabled_update_status()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(StorageState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _arguments, _directory| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            create_profile,
            login_profile,
            logout_profile,
            update_profile,
            load_data,
            save_data,
            list_backups,
            restore_backup,
            export_data,
            import_data,
            preview_statement,
            app_info,
            open_data_folder,
            updater_status,
            updater_check,
            updater_install
        ])
        .run(tauri::generate_context!())
        .expect("Nie udało się uruchomić programu Portfel");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_polish_statement_and_marks_duplicate() {
        let csv = "Data operacji;Opis transakcji;Kwota;Id transakcji\n20.07.2026;Sklep spożywczy;-123,45 PLN;ABC\n21.07.2026;Wypłata;5 000,00 zł;DEF\n";
        let first = parse_statement_text(csv, "konto.csv", "bank-1", &[]).unwrap();
        assert_eq!(first.rows.len(), 2);
        assert_eq!(first.rows[0].date, "2026-07-20");
        assert_eq!(first.rows[0].amount, -123.45);
        assert_eq!(first.rows[1].amount, 5000.0);

        let existing = vec![first.rows[0].fingerprint.clone()];
        let second = parse_statement_text(csv, "konto.csv", "bank-1", &existing).unwrap();
        assert!(second.rows[0].duplicate);
        assert!(!second.rows[1].duplicate);
    }

    #[test]
    fn migrates_old_cash_data_to_version_four() {
        let old = json!({
            "version": 3,
            "settings": {
                "currentYear": 2026,
                "theme": "light",
                "balances": { "2026": { "available": 1200, "reserve": 0 } },
                "categories": ["Inne"]
            },
            "goals": [], "debts": [], "recurring": [], "budgets": [], "transactions": []
        });
        let migrated = migrate_payload(old).unwrap();
        assert_eq!(migrated["version"], 5);
        assert_eq!(migrated["accounts"][0]["id"], "cash-main");
        assert_eq!(migrated["accounts"][0]["openingBalances"]["2026"], 1200.0);
        assert_eq!(migrated["settings"]["modules"]["bankAccounts"], false);
    }

    #[test]
    fn parses_quoted_csv_fields() {
        let rows = parse_csv("A;B\n1;\"tekst; z separatorem\"\n", ';');
        assert_eq!(rows[1][1], "tekst; z separatorem");
    }

    #[test]
    fn hashes_and_verifies_profile_password() {
        let encoded = hash_password("Bezpieczne hasło 2026")
            .unwrap()
            .expect("hasło powinno utworzyć skrót");
        assert_ne!(encoded, "Bezpieczne hasło 2026");
        assert!(verify_password("Bezpieczne hasło 2026", &encoded));
        assert!(!verify_password("inne hasło", &encoded));
        assert_eq!(hash_password("").unwrap(), None);
    }
}
