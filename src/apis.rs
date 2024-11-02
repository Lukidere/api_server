use crate::{api_key, connection, Buf, Client};

use base64::display;
use mysql::*;
use prelude::Queryable;
use std::net::Shutdown;
use std::sync::Arc;
use std::{error::Error, fmt::format};
use tokio::stream;
use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};
pub async fn main_menu(
    data: Client,
    mut stream: tokio::net::TcpStream,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    let id_usera = get_userid(data.username.clone(), connection.clone()).await;
    let menu: String = format!("Witaj na serwerze api:{}\nWybierz Opcje:\n1.Wyswielt swoje klucze api\n2.Dodaj Nowy klucz api\n3.Edytuj klucz api\n4.Usun klucz API\n", data.username.clone());
    let mut buf: Buf = [0; 32];
    let apis: Vec<api_key> = display_apis(id_usera, connection.clone()).await.unwrap();
    stream.write_all(menu.as_bytes()).await?;
    let bytes_read = stream.read(&mut buf).await?;
    match String::from_utf8_lossy(&buf[..bytes_read])
        .to_string()
        .trim()
        .to_string()
        .trim()
    {
        "1" => {
            for (index, api_key) in apis.iter().enumerate() {
                stream
                    .write_all(format!("{}|{}\n", index + 1, api_key.display()).as_bytes())
                    .await?
            }
        }
        "2" => {
            println!("wybrano dwa");
            insert_into_base(stream, data, connection).await?
        }
        "3" => {
            apis.iter()
                .enumerate()
                .for_each(|(index, key)| println!("{}|{}", index + 1, key.display()));
            buf.fill(0);
            stream
                .write_all(b"Wybierz klucz ktory chcesz edytowac")
                .await?;
            let bytes_read = stream.read(&mut buf).await?;
            if let Ok(index) = String::from_utf8_lossy(&mut buf[..bytes_read])
                .trim()
                .parse::<usize>()
            {
                if let Some(api) = apis.get(index - 1) {
                    buf.fill(0);
                    stream.write_all(b"Podaj nowa wartosc klucza").await?;
                    let bytes_read = stream.read(&mut buf).await?;
                    let new_data = String::from_utf8_lossy(&buf[..bytes_read])
                        .to_string()
                        .trim()
                        .to_string();
                    edit_key(&new_data, api.key.clone(), connection.clone()).await?;
                    stream
                        .write_all(b"Klucz pomyslnie zaaktualzowany\n")
                        .await?
                } else {
                    stream.write_all(b"Nieprawidlowy numer\n").await?
                }
            } else {
                stream.write_all(b"Nieprawidlowa opcja\n").await?
            }
        }
        "4" => {
            apis.iter()
                .enumerate()
                .for_each(|(index, key)| println!("{}|{}", index + 1, key.display()));
            buf.fill(0);
            stream
                .write_all(b"Wybierz klucz ktory chcesz edytowac")
                .await?;
            let bytes_read = stream.read(&mut buf).await?;
            if let Ok(index) = String::from_utf8_lossy(&mut buf[..bytes_read])
                .trim()
                .parse::<usize>()
            {
                if let Some(api) = apis.get(index - 1) {
                    buf.fill(0);
                    stream.write_all(b"Podaj nowa wartosc klucza").await?;
                    let bytes_read = stream.read(&mut buf).await?;
                    let new_data = String::from_utf8_lossy(&buf[..bytes_read])
                        .to_string()
                        .trim()
                        .to_string();
                    delete_key(api.key.clone(), connection.clone()).await?;
                    stream.write_all(b"Klucz pomyslnie usuniety\n").await?
                } else {
                    stream.write_all(b"Nieprawidlowy numer\n").await?
                }
            } else {
                stream.write_all(b"Nieprawidlowa opcja\n").await?
            }
        }
        &_ => {
            stream.write_all(b"Zla opcja").await;
        }
    }
    Ok(())
}

pub async fn display_apis(
    id: usize,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<Vec<api_key>, mysql::Error> {
    match connection.lock().await.query_map(
        format!("SELECT klucz,wartosc from klucze WHERE UserID = {}", id),
        |(key, data): (String, String)| api_key { key, data },
    ) {
        Ok(val) => Ok(val),
        Err(error) => return Err(error),
    }
}
pub async fn edit_key(
    new_data: &str,
    key: String,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    connection.lock().await.exec_drop(
        "UPDATE klucze SET wartosc = :new_value WHERE klucz = :key",
        params! {
            "new_value" => new_data,
            "key" => &key
        },
    );
    Ok(())
}
pub async fn delete_key(
    key: String,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    connection.lock().await.exec_drop(
        "DELETE FROM klucz where klucz = :key",
        params! {
            "key" => key
        },
    );
    Ok(())
}

pub async fn insert_into_base(
    mut stream: TcpStream,
    client: Client,
    connection: Arc<Mutex<PooledConn>>,
) -> Result<(), Box<dyn Error>> {
    let mut key = api_key::new(stream).await;
    let existing_keys = display_apis(
        get_userid(client.username.clone(), connection.clone()).await,
        connection.clone(),
    )
    .await?
    .iter()
    .map(|dana| dana.key.clone())
    .collect::<Vec<String>>();

    while existing_keys.contains(&key.key) {
        key = key.recreate_key();
    }
    let id_usera = get_userid(client.username, connection.clone()).await;
    println!("poczatek");
    connection.lock().await.exec_drop(
        "INSERT INTO klucze (UserID, klucz, wartosc) VALUES (:userid, :key, :value)",
        params! {
            "userid" => id_usera,
            "key" => &key.key,
            "value" => &key.data
        },
    )?;
    println!("koniec");

    Ok(())
}

pub async fn get_userid(user: String, connection: Arc<Mutex<PooledConn>>) -> usize {
    let query = format!("SELECT user_id FROM Users WHERE User = '{}'", user);
    match connection.lock().await.query::<String, String>(query) {
        Ok(val) => val.iter().next().unwrap().parse::<usize>().unwrap(),
        Err(_) => 0,
    }
}
