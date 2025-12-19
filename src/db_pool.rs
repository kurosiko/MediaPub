use deadpool_postgres::{Config, CreatePoolError, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

use crate::{POSTGRES_DBNAME, POSTGRES_HOST, POSTGRES_PASSWORD, POSTGRES_PORT, POSTGRES_USER};

pub fn create_pool() -> Result<Pool,CreatePoolError> {
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
