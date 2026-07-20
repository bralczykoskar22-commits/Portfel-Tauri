# Changelog

## 0.8.0-alpha.1

- pełne dopracowanie typografii, kart, formularzy i stanów interakcji,
- spójniejsze polskie teksty i komunikaty,
- centrum powiadomień zbierające terminy, przekroczone limity i plany wymagające uwagi,
- lepsza obsługa klawiatury, focus i trybu ograniczonych animacji,
- subtelne mikroanimacje oraz poprawione skalowanie mniejszych okien,
- wzmocniona konfiguracja pobierania zależności w GitHub Actions.

# Historia zmian

## 0.6.0-alpha.1

- przebudowano ekran wyboru profilu na profesjonalną kartę z czytelną listą,
  oznaczeniem profilu domyślnego i stanem zabezpieczenia,
- dodano pełnoekranowe, animowane tło inspirowane wykresem z Podsumowania:
  rysowaną linię, pulsujące punkty, unoszące się monety, siatkę i poświaty,
- dodano szybkie otwieranie profilu bez hasła oraz osobny formularz logowania
  dla profilu chronionego,
- dodano pokaż/ukryj hasło, zatwierdzanie Enterem, komunikaty błędów i płynne
  przejścia,
- dodano ustawienia wyglądu dostępne na ekranie startowym,
- dodano motywy Aurora, Ocean, Zachód i Szmaragd dla ekranu logowania oraz
  osobny wybór tła karty Podsumowanie,
- dodano możliwość ograniczenia animacji i zachowano jasny/ciemny tryb,
- uporządkowano Ustawienia w sześciu kategoriach: Profil, Wygląd, Budżet,
  Moduły, Dane i Program,
- rozszerzono długi o tryb „W ratach” lub „Bez rat — dowolne wpłaty”,
- termin całkowitej spłaty i konkretny dzień raty są teraz opcjonalne,
- dodano czytelne oznaczenia typu długu, rekomendowaną wpłatę i elastyczne
  prognozy,
- podniesiono schemat danych do wersji 6 i dodano bezpieczną migrację ustawień
  wyglądu oraz dotychczasowych długów,
- rozszerzono testy interfejsu o motywy, animowany ekran profili i elastyczne
  długi,
- zaktualizowano zależności testowe i usunięto znane podatności `npm audit`.

## 0.5.0-alpha.1

- dodano lokalne profile użytkowników z osobnymi danymi finansowymi,
- dodano profil domyślny i otwieranie profilu bez hasła jednym kliknięciem,
- dodano opcjonalne hasła profili haszowane Argon2 z losową solą,
- dodano animowany ekran startowy, wylogowanie oraz edycję profilu,
- rozszerzono operacje cykliczne o terminy, kwotę przewidywaną i faktyczną,
- dodano statusy Oczekuje, Zatwierdzona, Pominięta i Po terminie,
- rozdzielono kopie zapasowe pomiędzy profile.

## 0.4.0-alpha.1

- przywrócono kompletną logikę ostatniej wersji Electron,
- przywrócono konta, transfery, import CSV, cele, długi i limity,
- ujednolicono bazę w `%APPDATA%\Portfel\data\portfel.sqlite`,
- zwiększono historię do 30 kopii zapasowych.
