use crate::Buf;
use aead::{Aead, AeadCore, AeadMut, KeyInit, OsRng};
use argon2::Argon2;
use argon2::PasswordHash;
use argon2::password_hash::SaltString;
use argon2::{self,PasswordHasher,PasswordVerifier};
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
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
    pub hash:String,
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
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(plain.as_bytes(),&salt)
            .expect("hashing fail")
            .to_string();
        Pass {hash}

    }
    pub fn verify(&self,plain:&str) -> bool {
        let parsed = PasswordHash::new(&self.hash);
        if parsed.is_err() {
            return false
        }
        let parsed = parsed.unwrap();
        Argon2::default().verify_password(plain.as_bytes(),&parsed)
        .is_ok()
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
