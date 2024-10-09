use std::env;
use base64::{engine, Engine};

#[derive(Debug, Clone)]
pub(crate) struct Config {
    username: String,
    password: String
}
impl Config {
    pub fn new() -> Self {
        let encode = |v| engine::general_purpose::STANDARD.encode(v);
        Config {
            username: encode(env::var("CCTV_USERNAME").expect("Missing CCTV username env var!")),
            password: encode(env::var("CCTV_PASSWORD").expect("Missing CCTV password env var!"))
        }
    }

    #[inline]
    pub fn creds_match(&self, username: String, password: String) -> bool {
        let filter = |v: String| v.chars().filter(|&c| !c.is_control()).collect::<String>();
        filter(username) == self.username && filter(password) == self.password
    }
}