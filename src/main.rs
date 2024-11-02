mod connection;
mod structs;
mod user_handle;
use user_handle::handle;
type Buf = [u8; 32];
use connection::connect;
use mysql::{params, prelude::Queryable, Params, Value};
use std::{sync::Arc, thread};
use structs::*;
use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    sync::Mutex,
};
use warp::Filter;
mod apis;
const addr: &str = "127.0.0.1:2137";
#[tokio::main]
async fn main() {
    let mut connection = match connect() {
        Ok(val) => {
            println!("Połączenie udane :)");
            Arc::new(Mutex::new(val))
        }
        Err(_) => {
            panic!("Nie udalo sie połączyć z bazą danych");
        }
    };
    let api_keys: Vec<api_key> = match connection.lock().await.query_map(
        "SELECT klucz,wartosc FROM klucze",
        |(key, data)| api_key { key, data },
    ) {
        Ok(val) => {
            println!("udane wyciagniecie kluczy z bazy");
            val
        }
        Err(_) => panic!("Nie udalo sie wyciagnac kluczy z bazy"),
    };
    let api_keys_arc = Arc::new(api_keys);
    let api_key_filter =
        warp::header::optional::<String>("x-api-key").and_then(move |apikey: Option<String>| {
            let dane: Arc<Vec<api_key>> = Arc::clone(&api_keys_arc);
            async move {
                match apikey {
                    Some(ref key) => {
                        if let Some(api_key) = dane.iter().find(|dana| &dana.key == key) {
                            Ok(warp::reply::json(&api_key.data))
                        } else {
                            Err(warp::reject::custom(InvalidApiKey))
                        }
                    }
                    _ => Err(warp::reject::custom(InvalidApiKey)),
                }
            }
        });
    let api_key_localization = warp::path("api-key")
        .and(api_key_filter)
        .recover(handle_rejection);

    tokio::spawn(async {
        warp::serve(api_key_localization)
            .run(([127, 0, 0, 1], 3030))
            .await;
    });
    let server = TcpListener::bind(addr).await.unwrap();
    let dane: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(
        connection
            .lock()
            .await
            .query_map(
                "SELECT User,Password,key_cipher,nonce FROM Users",
                |(username, password_blob, key_cipher_blob, nonce_blob): (
                    String,
                    Vec<u8>,
                    Vec<u8>,
                    Vec<u8>,
                )| Client {
                    username,
                    password: Pass {
                        cipher: password_blob,
                        key: key_cipher_blob,
                        nonce: nonce_blob,
                    },
                },
            )
            .unwrap(),
    ));

    loop {
        for (mut klient, _) in server.accept().await {
            handle(klient, connection.clone()).await;
        }
    }
}
