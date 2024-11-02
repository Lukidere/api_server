use crate::Buf;
use aead::{Aead, AeadCore, AeadMut, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, ChaChaPoly1305, Key, Nonce};
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use rand::Rng;
use std::error::Error;
use std::io;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
pub struct api_key {
    pub key: String,
    pub data: String,
}

#[derive(Clone)]
pub struct Client {
    pub username: String,
    pub password: Pass,
}

#[derive(Clone)]
pub struct Pass {
    pub cipher: Vec<u8>,
    pub key: Vec<u8>,
    pub nonce: Vec<u8>,
}

impl api_key {
    pub async fn new(mut klient: TcpStream) -> api_key {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        let mut buf: Buf = [0; 32];
        klient
            .write(format!("Podaj wartosc ktora ma byc przypisana do klucza:{}\n", s).as_bytes())
            .await;
        let bytes_read = klient.read(&mut buf).await.unwrap();

        let dane = String::from_utf8_lossy(&buf[..bytes_read]);
        api_key {
            key: s,
            data: dane.to_string().trim().to_string(),
        }
    }
    pub fn display(&self) -> String {
        format!("Key:{},Value:{}", &self.key, &self.data)
    }
    pub fn recreate_key(self) -> api_key {
        api_key {
            key: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(12)
                .map(char::from)
                .collect(),
            data: self.data,
        }
    }
}

impl Pass {
    pub fn new(plain: String) -> Pass {
        let key = ChaCha20Poly1305::generate_key(OsRng);
        let nonce = ChaCha20Poly1305::generate_nonce(OsRng);
        let cipher = ChaCha20Poly1305::new(&key);
        let haslo = cipher.encrypt(&nonce, plain.as_bytes()).unwrap();
        Pass {
            cipher: haslo,
            key: key.to_vec(),
            nonce: nonce.to_vec(),
        }
    }
    pub fn read(&mut self) -> Result<String, Box<dyn Error>> {
        let key: Key = Key::from_slice(self.key.as_slice()).to_owned();
        let nonce: Nonce = Nonce::from_slice(&self.nonce.as_slice()).to_owned();
        let cipher = ChaCha20Poly1305::new(&key);
        let decrypted = cipher.decrypt(&nonce, self.cipher.as_slice()).unwrap();
        self.nonce = ChaCha20Poly1305::generate_nonce(OsRng).to_vec();
        Ok(String::from_utf8(decrypted).unwrap())
    }
}
pub trait StringExt {
    fn clean_out(&self) -> String;
}
impl StringExt for String {
    fn clean_out(&self) -> String {
        self.chars()
            .filter(|&c| c != '\0')
            .collect::<String>()
            .trim()
            .to_string()
            .to_owned()
    }
}
impl Client {
    pub async fn new(mut socket: TcpStream) -> (Client, tokio::net::TcpStream) {
        let mut buf: Buf = [0; 32];
        let _ = socket.write("Proszę podaj swój nick \n".as_bytes()).await;
        let bytes_read = socket.read(&mut buf).await.unwrap();
        let nickname = String::from_utf8_lossy(&mut buf[..bytes_read])
            .trim()
            .to_string();
        let _ = socket
            .write("Prosze podaj teraz swoje haslo: \n".as_bytes())
            .await;
        buf.fill(0);
        let bytes_read = socket.read(&mut buf).await.unwrap();
        let pass = String::from_utf8_lossy(&mut buf[..bytes_read]).to_string();
        (
            Client {
                username: nickname.clean_out(),
                password: Pass::new(pass.trim().to_string()),
            },
            socket,
        )
    }
}
#[derive(Debug)]
pub struct InvalidApiKey;

impl warp::reject::Reject for InvalidApiKey {}

pub async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if err.find::<InvalidApiKey>().is_some() {
        Ok(warp::reply::with_status(
            "Invalid API key",
            warp::http::StatusCode::UNAUTHORIZED,
        ))
    } else {
        Err(err)
    }
}
