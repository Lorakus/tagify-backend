use serde::{Deserialize, Serialize};
use tokio_pg_mapper_derive::PostgresMapper;

// used for password hashing
use argon2::Config;
use rand::Rng;

#[derive(Serialize)]
pub struct Status {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PostgresMapper)]
#[pg_mapper(table = "users")]
pub struct User {
    pub id: i32,
    pub username: String,
    pub nickname: String,
    pub password: String,
    pub role: String, // TODO: Make an Enum out of it
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserAdmin {
    pub nickname: String,
    pub password: String,
    pub role: String, // TODO: Make an Enum out of it
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub nickname: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendUser {
    pub id: i32,
    pub username: String,
    pub nickname: String,
    pub role: String, // TODO: Make an Enum out of it
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub role: String, // TODO: Make an Enum out of it
}

// Hash password, can be implemented for Structs containing .passwort attribut
pub trait Hash {
    fn hash_password(&mut self) -> Result<(), argon2::Error>;
    fn get_hashed_password(&self) -> Result<String, argon2::Error>;

    #[allow(dead_code)]
    fn verify_password(&self, password: &[u8]) -> Result<bool, argon2::Error>;
}

//Hash implementation for User & password in one Trait

impl Hash for User {
    fn get_hashed_password(&self) -> Result<String, argon2::Error> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        Ok(argon2::hash_encoded(
            self.password.as_bytes(),
            &salt,
            &config,
        )?)
    }

    fn hash_password(&mut self) -> Result<(), argon2::Error> {
        self.password = self.get_hashed_password()?;
        Ok(())
    }

    #[allow(dead_code)]
    fn verify_password(&self, password: &[u8]) -> Result<bool, argon2::Error> {
        Ok(argon2::verify_encoded(&self.password, password)?)
    }
}

impl Hash for LoginData {
    fn get_hashed_password(&self) -> Result<String, argon2::Error> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        Ok(argon2::hash_encoded(
            self.password.as_bytes(),
            &salt,
            &config,
        )?)
    }

    fn hash_password(&mut self) -> Result<(), argon2::Error> {
        self.password = self.get_hashed_password()?;
        Ok(())
    }

    #[allow(dead_code)]
    fn verify_password(&self, password: &[u8]) -> Result<bool, argon2::Error> {
        Ok(argon2::verify_encoded(&self.password, password)?)
    }
}

impl Hash for CreateUser {
    fn get_hashed_password(&self) -> Result<String, argon2::Error> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        Ok(argon2::hash_encoded(
            self.password.as_bytes(),
            &salt,
            &config,
        )?)
    }

    fn hash_password(&mut self) -> Result<(), argon2::Error> {
        self.password = self.get_hashed_password()?;
        Ok(())
    }

    #[allow(dead_code)]
    fn verify_password(&self, password: &[u8]) -> Result<bool, argon2::Error> {
        Ok(argon2::verify_encoded(&self.password, password)?)
    }
}

// TODO: Make this a Sql serializable enum
pub const ROLES: &'static [&'static str] = &["admin", "user"];
