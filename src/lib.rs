pub mod db_pool;
pub mod errors;
pub mod init;
pub mod route;
pub mod types;
pub mod utility;

pub const DESTINATION: &str = "./tmp";
pub const MAX_PAYLOAD_SIZE: usize = 1024 * 1024 * 1024;
pub const DB_PATH: &str = "./data/database.db";
pub const MONGODB_URI: &str = "mongodb://localhost:27017";
pub const POSTGRES_HOST: &str = "localhost";
pub const POSTGRES_PORT: u16 = 5432;
pub const POSTGRES_USER: &str = "postgres";
pub const POSTGRES_PASSWORD: &str = "ahogehub";
pub const POSTGRES_DBNAME: &str = "postgres";
pub const ACTIX_PORT: u16 = 8080;
pub const ACTIX_SERVER:&str = "0.0.0.0";
//these configration is for development environment