use crate::apis::main_menu;
use crate::{Buf, Client, Pass};
use mysql::{params,prelude::Queryable, PooledConn, Value};
use std::error::Error;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};


pub async fn handle(
    mut client: TcpStream,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    client
        .write_all(b"Welcome to the api_key server user management\n")
        .await?;
    client
        .write_all(b"Choose one option:\n1) Login\n2) Register\n")
        .await?;

    let mut buf = [0; 32];
    let bytes_read = client.read(&mut buf).await?;

    if bytes_read == 0 {
        client.write_all(b"Blad: brak wejscia\n").await?;
        return Ok(());
    }

    let input = String::from_utf8_lossy(&buf[..bytes_read]).trim().to_string();

    match input.as_str() {
        "1" => {
            buf.fill(0);
            client
                .write_all(b"Podaj swoja nazwe uzytkownika\n")
                .await?;

            let username_bytes = client.read(&mut buf).await?;
            if username_bytes == 0 {
                client.write_all(b"Blad: brak nazwy uzytkownika\n").await?;
                return Ok(());
            }

            let szukany = String::from_utf8_lossy(&buf[..username_bytes]).trim().to_string();

            match wyszukaj_klienta(connection.clone(), szukany).await {
                Ok(user_data) => {
                    // ważne: propagate błędy (?)
                    login(client, connection.clone(), user_data).await?;
                }
                Err(_) => {
                    client
                        .write_all(b"Nie znaleziono, wybieram opcje rejestracji...\n")
                        .await?;
                    register(client, connection.clone()).await?;
                }
            }
        }
        "2" => {
            register(client, connection.clone()).await?;
        }
        _ => {
            client.write_all(b"Niepoprawny wybor\n").await?;
        }
    }

    Ok(())
}

pub async fn login(
    mut client: TcpStream,
    connection: Arc<Mutex<PooledConn>>,
    porownawcze: Client,
) -> Result<Client, Box<dyn Error>> {
    let mut buf: Buf = [0; 32];

    client.write_all(b"Podaj haslo \n").await?;
    let bytes_read = client.read(&mut buf).await?;
    if bytes_read == 0 {
        client.write_all(b"Blad: brak hasla\n").await?;
        return Ok(porownawcze);
    }

    let input = String::from_utf8_lossy(&buf[..bytes_read]).trim().to_string();

    if porownawcze.password.verify(&input) {
        client
            .write_all(format!("Witamy ponownie {}", porownawcze.username).as_bytes())
            .await?;
        main_menu(porownawcze.clone(), client, connection).await?;
    } else {
        client.write_all(b"Niepoprawne haslo!").await?;
    }

    Ok(porownawcze)
}

pub async fn register(
    client: TcpStream,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    let (dane, mut client) = Client::new(client).await;

    // jeśli user istnieje -> przekieruj do login
    if let Ok(user) = wyszukaj_klienta(connection.clone(), dane.username.clone()).await {
        client
            .write_all(b"Twoj uzytkownik juz istnieje przekierowuje do logowania....")
            .await?;
        login(client, connection, user).await?;
        return Ok(());
    }

    // zapis: w Users.Password trzymamy dane.password.hash (String)
    connection.lock().await.exec_drop(
        r#"INSERT INTO Users (user_id, User, Password) VALUES (NULL, ?, ?)"#,
        vec![
            Value::from(dane.username.replace(' ', "")),
            Value::from(dane.password.hash.clone()),
        ],
    )?;

    client
        .write_all(b"udalo sie zarejestrowac uzytkownika")
        .await?;

    Ok(())
}

async fn wyszukaj_klienta(
    connection: Arc<Mutex<PooledConn>>,
    szukany: String,
) -> Result<Client, bool> {
    let mut conn = connection.lock().await;

    // Parametryzowane zapytanie: brak SQL injection
    let row: Option<(String, String)> = conn
        .exec_first(
            "SELECT User, Password FROM Users WHERE User = :user LIMIT 1",
            params! { "user" => szukany },
        )
        .map_err(|_| false)?;

    match row {
        Some((username, password_hash)) if !password_hash.is_empty() => Ok(Client {
            username,
            password: Pass { hash: password_hash },
        }),
        _ => Err(false),
    }
}
