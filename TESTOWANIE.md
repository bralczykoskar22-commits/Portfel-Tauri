# Testowanie wersji 0.6.0-alpha.1

## 1. Ekran startowy i profile

1. Uruchom aplikację i sprawdź płynną animację wykresu, punktów oraz monet.
2. Kliknij ikonę koła zębatego, wybierz inny motyw i sprawdź zmianę tła.
3. Otwórz profil bez hasła jednym kliknięciem.
4. Utwórz profil z hasłem. Wyloguj się i sprawdź, że kliknięcie profilu pokazuje
   formularz hasła, przycisk pokaż/ukryj działa, a Enter otwiera profil.
5. Wpisz nieprawidłowe hasło i sprawdź czytelny komunikat błędu.
6. Zmień nazwę, kolor, zdjęcie i hasło profilu. Ustaw profil jako domyślny.
7. Zamknij i uruchom program ponownie; profile i oznaczenie domyślne powinny
   pozostać zachowane.
8. Dodaj różne operacje w dwóch profilach i potwierdź, że dane się nie mieszają.

## 2. Motywy i ustawienia

1. Otwórz Ustawienia i przejdź kolejno przez sześć kategorii.
2. W kategorii Wygląd wybierz osobne motywy logowania i Podsumowania.
3. Przełącz tryb jasny/ciemny oraz wyłącz animacje.
4. Zapisz, wyloguj się i sprawdź zachowanie ustawień na ekranie startowym.
5. Potwierdź czytelność ustawień na mniejszym oknie aplikacji.

## 3. Długi

1. Dodaj dług ratalny z terminem, kwotą raty i konkretnym dniem miesiąca.
2. Dodaj dług ratalny bez stałego dnia raty.
3. Dodaj dług „Bez rat — dowolne wpłaty” z terminem końcowym.
4. Dodaj dług elastyczny bez terminu końcowego.
5. Sprawdź oznaczenia „Raty”/„Bez rat”, termin, rekomendowaną wpłatę i prognozę.
6. Dodaj spłatę, zapisz dane, uruchom program ponownie i sprawdź saldo długu.

## 4. Operacje cykliczne

1. Dodaj operację z datą rozpoczęcia i zakończenia oraz drugą bezterminową.
2. Dla dzisiejszej lub zaległej realizacji kliknij „Zatwierdź” i wpisz kwotę
   faktyczną inną od przewidywanej.
3. Sprawdź status „Zatwierdzona” i kwotę faktyczną w podsumowaniu.
4. Pomiń inną realizację i sprawdź status „Pominięta”.
5. Pozostaw zaległą realizację bez decyzji i sprawdź status „Po terminie”.
6. Potwierdź, że prognoza używa kwoty przewidywanej.

## 5. Dane i regresja

1. Kliknij „Zapisz”, zamknij i ponownie uruchom aplikację.
2. Sprawdź eksport/import JSON, automatyczne kopie i folder danych.
3. Opcjonalnie przetestuj konta, transfery i import wyciągu CSV.
4. Sprawdź, czy dotychczasowe dane z wersji 0.5 zostały zachowane.

W zgłoszeniu błędu podaj profil, motyw, zakładkę, dokładne kroki, oczekiwany i
faktyczny wynik oraz dołącz zrzut ekranu.
