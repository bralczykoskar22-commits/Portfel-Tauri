# Portfel Tauri

Nowe repozytorium aplikacji **Portfel** rozwijanej w Tauri.

## Aktualny stan

Wersja robocza: **0.5.0-alpha.1**.

Gotowa lokalna paczka źródłowa została przygotowana poza repozytorium jako kopia awaryjna. Zawiera pełny kod Tauri, patch zmian, instrukcję budowania, raport kontroli i sumy SHA-256.

## Zakres wersji 0.5.0-alpha.1

- lokalne profile użytkowników,
- opcjonalne hasło profilu,
- osobne bazy SQLite dla profili,
- animowany ekran wyboru profilu,
- harmonogramy cykliczne z datą końcową albo trybem bezterminowym,
- kwota przewidywana i faktyczna przy potwierdzaniu wpływu lub wydatku,
- zachowanie wcześniejszej bazy jako konta „Moje konto”.

## Budowanie

Po wgraniu pełnych źródeł wymagane będą:

```text
npm ci
npm run audit
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run build
```

## Uwaga

Import pełnych źródeł do GitHuba jest kontynuowany ostrożnie, ponieważ poprzednia metoda dzielenia dużych plików była zbyt wolna. Główna kopia awaryjna ZIP pozostaje dostępna lokalnie w rozmowie jako niezależna paczka projektu.
