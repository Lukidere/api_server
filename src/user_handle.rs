use crate::apis::main_menu;
use crate::{connection, Buf, Client};
use crate::{Pass, StringExt};
use aead::{Aead, AeadCore, AeadMut, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, ChaChaPoly1305, Key, Nonce};
use mysql::Value;
use mysql::{prelude::Queryable, PooledConn};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::broadcast::error;
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

    if bytes_read > 0 {
        let input = String::from_utf8_lossy(&buf[..bytes_read])
            .trim()
            .to_string();

        match input.as_str() {
            "1" => {
                buf.fill(0);
                client.write_all(b"Podaj swoja nazwe uzytkownika\n").await?;

                // Read username
                let username_bytes = client.read(&mut buf).await?;
                if username_bytes > 0 {
                    let szukany = String::from_utf8_lossy(&buf[..username_bytes])
                        .trim()
                        .to_string();
                    let wyszukany = wyszukaj_klienta(connection.clone(), szukany).await;

                    if let Ok(user_data) = wyszukany {
                        login(client, connection.clone(), user_data).await;
                    } else {
                        client
                            .write_all(b"Nie znaleziono, wybieram opcje rejestracji...\n")
                            .await?;
                        register(client, connection.clone()).await;
                    }
                }
            }
            "2" => {
                register(client, connection.clone()).await;
            }
            _ => {
                client.write_all(b"Niepoprawny wybor\n").await?;
            }
        }
    } else {
        client.write_all(b"Blad: brak wejscia\n").await?;
    }
    Ok(())
}

pub async fn login(
    mut client: tokio::net::TcpStream,
    connection: Arc<Mutex<PooledConn>>,
    mut porownawcze: Client,
) -> Result<Client, Box<dyn Error>> {
    let mut buf: Buf = [0; 32];
    client.write_all(b"Podaj haslo \n").await?;
    let bytes_read = client.read(&mut buf).await?;
    let input = String::from_utf8_lossy(&buf[..bytes_read])
        .trim()
        .to_string();
    let key = Key::from_slice(&porownawcze.password.key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&porownawcze.password.nonce);
    let wannabepass = cipher.encrypt(&nonce, input.as_bytes()).unwrap();
    if wannabepass.as_slice() == porownawcze.password.cipher.as_slice() {
        client
            .write(format!("Witamy ponownie {}", porownawcze.username).as_bytes())
            .await;
        main_menu(porownawcze.clone(), client, connection).await?;
    } else {
        client.write(b"Niepoprawne haslo!").await?;
    }
    Ok(porownawcze)
}

pub async fn register(
    mut client: tokio::net::TcpStream,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    let (dane, mut client) = Client::new(client).await;
    if let Ok(user) = wyszukaj_klienta(connection.clone(), dane.username.clone()).await {
        client
            .write_all(b"Twoj uzytkownik juz istnieje przekierowuje do logowania....")
            .await?;
        login(client, connection, user).await?;
        Ok(())
    } else {
        match connection.lock().await.exec::<String, &str, Vec<Value>>(
            r#"INSERT INTO Users (user_id,User,Password,key_cipher,nonce) VALUES (NULL,?,?,?,?)"#,
            vec![
                Value::from(dane.username.to_string().replace(" ", "")),
                Value::from(dane.password.cipher.clone()),
                Value::from(dane.password.key),
                Value::from(dane.password.nonce),
            ],
        ) {
            Ok(val) => {
                client
                    .write_all(b"udalo sie zarejestrowac uzytkownika")
                    .await;
                return Ok(());
            }
            Err(blad) => {
                client
                    .write_all(format!("wystapil blad podczas rejestracji:{}", blad).as_bytes())
                    .await;
                return Err(Box::new(blad));
            }
        }
    }
}

async fn wyszukaj_klienta(
    connection: Arc<Mutex<PooledConn>>,
    szukany: String,
) -> Result<Client, bool> {
    let checked: String = String::from("");
    let query = format!(
        "SELECT User,Password,key_cipher,nonce FROM Users WHERE User = '{}'",
        szukany
    );
    let dane: Client = match connection
        .lock()
        .await
        .query_map(
            &query,
            |(username, passcode, key_cipher, nonce): (String, Vec<u8>, Vec<u8>, Vec<u8>)| Client {
                username,
                password: Pass {
                    cipher: passcode,
                    key: key_cipher,
                    nonce,
                },
            },
        )
        .unwrap()
        .into_iter()
        .next()
    {
        Some(val) => val,
        _ => return Err(false),
    };
    match dane.password.cipher.len() {
        0 => return Err(false),
        _ => return Ok(dane),
    }
}
