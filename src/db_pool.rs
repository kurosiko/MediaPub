use deadpool_postgres::{Config, CreatePoolError, ManagerConfig, Pool, RecyclingMethod, Runtime};
use mongodb::{Client, error::Error as MongoError, options::{ClientOptions, Credential, ServerAddress}};
use tokio_postgres::NoTls;

use crate::{MONGODB_HOST, MONGODB_PASSWORD, MONGODB_PORT, MONGODB_USER, POSTGRES_DBNAME, POSTGRES_HOST, POSTGRES_PASSWORD, POSTGRES_PORT, POSTGRES_USER};

pub async fn create_psql_pool() -> Result<Pool,CreatePoolError> {
    let mut cfg = Config::new();
    cfg.user = Some(POSTGRES_USER.to_string());
    cfg.password = Some(POSTGRES_PASSWORD.to_string());
    cfg.host = Some(POSTGRES_HOST.to_string());
    cfg.port = Some(POSTGRES_PORT);
    cfg.dbname = Some(POSTGRES_DBNAME.to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    Ok(pool)
}

pub async fn create_mongo_pool() -> Result<Client,MongoError>{
    let options = ClientOptions::builder()
        .hosts(vec![ServerAddress::Tcp {host: MONGODB_HOST.to_string(),port: Some(MONGODB_PORT)}])
        .credential(
            Credential::builder()
                .username(Some(MONGODB_USER.to_string()))
                .password(Some(MONGODB_PASSWORD.to_string()))
                .source(Some("admin".to_string()))
                .build(),
        )
        .max_pool_size(Some(50))
        .min_pool_size(Some(5))
        .build();
    let client = Client::with_options(options)?;
    Ok(client)
}