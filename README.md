# Portfel — wersja Tauri

Prywatna, lokalna aplikacja finansowa dla Windows 10/11. Interfejs zachowuje
układ, animacje oraz jasny i ciemny motyw poprzedniej wersji, a dane pozostają
wyłącznie na komputerze użytkownika.

## Najważniejsze funkcje

- lokalne profile użytkowników z osobnymi danymi finansowymi,
- profil bez hasła lub chroniony hasłem przechowywanym jako skrót Argon2,
- profil domyślny, animowany ekran wyboru profilu i przycisk „Wyloguj”,
- zmiana nazwy, koloru, zdjęcia i hasła aktywnego profilu,
- operacje cykliczne z datą rozpoczęcia, datą końcową albo trybem bezterminowym,
- kwota przewidywana oraz zatwierdzana kwota faktyczna dla każdej realizacji,
- status realizacji: Oczekuje, Zatwierdzona, Pominięta albo Po terminie,
- prognozy liczone z kwot przewidywanych, podsumowania z zatwierdzonych operacji,
- widoki miesięczne i roczne, cele, długi, budżety kategorii i alerty,
- opcjonalne konta gotówkowe, bankowe i oszczędnościowe,
- transfery pomiędzy kontami bez zaliczania ich do wydatków,
- lokalny import wyciągów CSV z podglądem i wykrywaniem duplikatów,
- lokalna baza SQLite oraz 30 automatycznych kopii dla każdego profilu,
- eksport i import danych JSON.

Program nie używa `localStorage`, nie uruchamia serwera i nie wymaga rejestracji
internetowej. Hasło blokuje otwarcie profilu w aplikacji; baza finansowa nie jest
szyfrowana na dysku.

## Dane programu i migracja

Na Windows baza jest tworzona standardowo tutaj:

```text
%APPDATA%\Portfel\data\portfel.sqlite
```

Przy pierwszym uruchomieniu wersji 0.5 istniejące dane są automatycznie
przypisywane do lokalnego profilu „Mój profil”. Każdy kolejny profil ma osobny
rekord danych i osobny katalog kopii zapasowych. Odinstalowanie aplikacji nie
powinno usuwać bazy, ale przed większą aktualizacją warto użyć opcji „Eksportuj
kopię”.

## Uruchomienie gotowej wersji

Najwygodniej uruchomić instalator `Portfel_*_x64-setup.exe`. Wersja przenośna to
`Aplikacja\Portfel.exe`. Aplikacja korzysta z Microsoft Edge WebView2.

## Praca z kodem

Wymagane są Node.js 20+, Rust stable oraz na Windows narzędzia C++ z Visual
Studio Build Tools.

```text
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Można też uruchomić `build-windows.bat`.

## Struktura projektu

- `src/` — interfejs HTML/CSS/JS oraz most poleceń Tauri,
- `src-tauri/` — okno natywne, profile, Argon2, SQLite, kopie i import CSV,
- `tests/` — automatyczne testy interfejsu,
- `.github/workflows/windows-build.yml` — testy i budowa Windows.

Katalogi `node_modules/` i `src-tauri/target/` są generowane i nie należą do
kodu źródłowego.
