# Raport kontroli — Portfel 0.8.0-alpha.1

## Zrealizowany zakres

- profesjonalna karta wyboru profilu,
- szybkie logowanie profilu bez hasła,
- osobny formularz hasła z pokaż/ukryj i obsługą Enter,
- animowane tło logowania inspirowane wykresem z Podsumowania,
- cztery motywy logowania i Podsumowania,
- możliwość ograniczenia animacji,
- uporządkowane ustawienia podzielone na sześć kategorii,
- długi ratalne lub elastyczne,
- opcjonalny termin całkowitej spłaty,
- opcjonalny konkretny dzień raty,
- migracja danych do schematu 6.

## Wykonane kontrole

- `npm ci` — zakończone powodzeniem,
- `npm audit --audit-level=high` — 0 podatności,
- `npm test` — 3/3 testy zakończone powodzeniem,
- `node --check src/assets/app.js` — poprawna składnia,
- kontrola JSON — wszystkie pliki poprawne,
- kontrola YAML workflow — plik poprawnie odczytany,
- kontrola HTML — 272 identyfikatory, brak duplikatów,
- kontrola CSS — zgodna liczba nawiasów blokowych.

Testy obejmują również profil chroniony błędnym i poprawnym hasłem, szybkie
otwarcie profilu bez hasła, motywy wyglądu oraz dług elastyczny bez terminu.

## Kompilacja Windows

Lokalne środowisko przygotowujące paczkę nie zawiera kompilatora Rust. Pełną
kontrolę Rust i budowę instalatora wykonuje dołączony workflow GitHub Actions:

```text
.github/workflows/windows-build.yml
```

Po przesłaniu paczki do gałęzi `main` workflow uruchamia `cargo test` oraz
`npm run build`. Gotowy wynik znajduje się w artefakcie
`Portfel-Tauri-Windows-x64-v0.8.0-alpha.1`.
