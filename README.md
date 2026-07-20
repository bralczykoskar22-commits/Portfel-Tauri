# Portfel — Tauri 0.6.0-alpha.1

Prywatna aplikacja finansowa dla Windows 10/11. Dane finansowe są zapisywane
lokalnie w SQLite i rozdzielone pomiędzy profile użytkowników. Program nie
wymaga rejestracji ani połączenia z usługą chmurową.

## Najważniejsze funkcje

### Profile użytkowników

- profesjonalny ekran wyboru profilu z animowanym wykresem i monetami,
- profil bez hasła otwierany jednym kliknięciem,
- profil chroniony hasłem z formularzem pokaż/ukryj i obsługą klawisza Enter,
- oznaczenie profilu domyślnego,
- osobne dane finansowe i kopie zapasowe każdego profilu,
- zmiana nazwy, koloru, zdjęcia, hasła i profilu domyślnego,
- przycisk „Wyloguj” dostępny w aplikacji,
- hasła przechowywane jako skróty Argon2 z losową solą, nigdy jako zwykły tekst.

### Wygląd

- cztery warianty tła: Aurora, Ocean, Zachód i Szmaragd,
- osobny wybór tła ekranu logowania i karty Podsumowanie,
- tryb jasny i ciemny,
- możliwość ograniczenia animacji,
- ustawienia wyglądu dostępne także przed otwarciem profilu.

### Operacje cykliczne

- data rozpoczęcia,
- data zakończenia albo tryb „Bezterminowo”,
- kwota przewidywana i osobna kwota faktyczna przy zatwierdzaniu,
- statusy: Oczekuje, Zatwierdzona, Pominięta i Po terminie,
- prognozy oparte na kwocie przewidywanej,
- podsumowania oparte na faktycznie zatwierdzonych operacjach.

### Długi

- plan ratalny albo elastyczne wpłaty bez rat,
- opcjonalny termin całkowitej spłaty,
- opcjonalny stały dzień raty,
- kwota raty lub planowana miesięczna wpłata,
- opcjonalne oprocentowanie i prognoza odsetek,
- rekomendowana kwota oraz prognozowany termin spłaty.

### Pozostałe

- widoki miesięczne i roczne, cele, koperty kategorii i alerty,
- opcjonalne konta gotówkowe, bankowe i oszczędnościowe,
- transfery pomiędzy kontami,
- lokalny import wyciągów CSV z podglądem i wykrywaniem duplikatów,
- 30 automatycznych kopii zapasowych dla każdego profilu,
- eksport i import danych JSON,
- ustawienia podzielone na kategorie: Profil, Wygląd, Budżet, Moduły, Dane i Program.

## Prywatność i dane

Dane finansowe nie są przechowywane w `localStorage`. Niewrażliwy wybór motywu
ekranu startowego jest zapamiętywany lokalnie, aby można go było zastosować
jeszcze przed otwarciem profilu. Baza finansowa znajduje się standardowo tutaj:

```text
%APPDATA%\Portfel\data\portfel.sqlite
```

Przy aktualizacji ze starszej wersji istniejące dane są automatycznie
przypisywane do profilu „Mój profil”. Baza nie jest szyfrowana na dysku — hasło
chroni wejście do profilu w aplikacji. Przed większą aktualizacją warto wykonać
eksport kopii JSON.

## Budowanie na GitHubie

Plik `.github/workflows/windows-build.yml` automatycznie uruchamia testy na
Windows i tworzy artefakt zawierający:

```text
Aplikacja/Portfel.exe
Instalator/Portfel_*_x64-setup.exe
Kod-zrodlowy/Portfel-Tauri/
```

Po wysłaniu zmian do gałęzi `main` przejdź do **Actions**, otwórz zakończony
przepływ „Portfel Tauri — Windows” i pobierz artefakt
`Portfel-Tauri-Windows-x64-v0.6.0-alpha.1`.

## Budowanie lokalne

Wymagane są Node.js 20+, Rust stable i narzędzia C++ Visual Studio Build Tools.

```text
npm ci
npm audit --audit-level=high
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Na Windows można również uruchomić `build-windows.bat`.

## Struktura projektu

- `src/` — interfejs HTML/CSS/JavaScript,
- `src-tauri/` — Tauri, profile, Argon2, SQLite, kopie i import CSV,
- `tests/` — automatyczne testy interfejsu,
- `.github/workflows/windows-build.yml` — testy i budowa instalatora Windows.
