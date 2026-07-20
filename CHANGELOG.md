# Historia zmian

## 0.5.0-alpha.1

- dodano lokalne profile użytkowników z osobnymi danymi finansowymi,
- dodano profil domyślny i otwieranie profilu bez hasła jednym kliknięciem,
- dodano opcjonalne hasła profili haszowane Argon2 z losową solą,
- dodano animowany ekran startowy, wylogowanie oraz edycję nazwy, koloru,
  zdjęcia i hasła,
- dotychczasowe dane są automatycznie migrowane do profilu „Mój profil”,
- rozszerzono operacje cykliczne o datę końcową lub tryb bezterminowy,
- rozdzielono kwotę przewidywaną od faktycznej kwoty zatwierdzenia,
- dodano statusy Oczekuje, Zatwierdzona, Pominięta i Po terminie,
- prognozy używają kwot przewidywanych, a podsumowania zapisanych operacji,
- kopie zapasowe są przechowywane osobno dla każdego profilu,
- rozszerzono automatyczne testy interfejsu i migracji danych.

## 0.4.0-alpha.1

- przywrócono kompletną logikę ostatniej wersji Electron,
- przywrócono konta, transfery, import CSV, cele, długi i limity,
- ujednolicono bazę w `%APPDATA%\Portfel\data\portfel.sqlite`,
- zwiększono historię do 30 plikowych kopii zapasowych,
- dodano eksport/import JSON, folder danych i pojedynczą instancję.

## 0.3.0-alpha.1

- rozpoczęto migrację aplikacji z Avalonia na Tauri 2,
- zastąpiono zapis JSON natywną bazą SQLite,
- przygotowano instalator NSIS oraz kontrolną kompilację Windows.
