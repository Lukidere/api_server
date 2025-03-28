# API Serwer w Rust

Ten projekt to lekki serwer API napisany w Rust, zaprojektowany do obsługi kluczy uwierzytelniających i interakcji z bazą danych SQL. Obsługuje również zarządzanie kontami użytkowników oraz szyfrowanie haseł.

## Konfiguracja i Ustawienia

Przed uruchomieniem serwera należy skonfigurować następujące pliki:

### 1. `skrypt.sh`
Dostosuj ten skrypt do swoich wymagań wdrożeniowych, takich jak ustawianie zmiennych środowiskowych lub konfiguracja poleceń startowych.

### 2. `connection.rs`
Zaktualizuj ten plik, aby określić poprawne parametry połączenia z bazą danych SQL.

### 3. `main.rs`
W **linii 62** pliku `main.rs` ustaw odpowiedni adres do łączenia się w celu pobrania klucza.

## Wymagania
- Rust (zalecana najnowsza stabilna wersja)
- Baza danych SQL (PostgreSQL, MySQL lub SQLite, w zależności od konfiguracji)

## Uruchamianie Serwera
Po dokonaniu konfiguracji uruchom serwer za pomocą:
```sh
cargo run --release
```

## Obsługa kont i szyfrowanie haseł
Serwer obsługuje rejestrację oraz zarządzanie kontami użytkowników. Hasła są przechowywane w postaci zaszyfrowanej zgodnie z najlepszymi praktykami bezpieczeństwa.

## Pobieranie Klucza API
Aby uzyskać klucz API, połącz się z skonfigurowanym adresem (ustawionym w `main.rs` w linii 62) i postępuj zgodnie z procesem uwierzytelniania.

## Licencja
Ten projekt jest licencjonowany na podstawie licencji MIT.

## Współtworzenie
Zachęcamy do zgłaszania problemów i przesyłania pull requestów w celu ulepszenia projektu!

