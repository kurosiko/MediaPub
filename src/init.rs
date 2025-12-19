use crate::utility;
use deadpool_postgres::Pool;
use std::io::{Error, ErrorKind, Result};

const POSTGRES_SQL: &str = "
CREATE TABLE IF NOT EXISTS \"user\" (
    user_id UUID PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expired_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    CONSTRAINT username_length CHECK (LENGTH(username) >= 3 AND LENGTH(username) <= 255)
);

CREATE TABLE IF NOT EXISTS \"session\" (
    token_id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES \"user\"(user_id) ON DELETE CASCADE,
    session_token TEXT NOT NULL UNIQUE,
    refresh_token_hash TEXT NOT NULL,
    session_expires_at TIMESTAMPTZ NOT NULL,
    refresh_expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_revoked BOOLEAN NOT NULL DEFAULT false,
    ip_address INET,
    user_agent TEXT,
    CONSTRAINT valid_session_expiry CHECK (session_expires_at > created_at),
    CONSTRAINT valid_refresh_expiry CHECK (refresh_expires_at > created_at)
);

CREATE TABLE IF NOT EXISTS \"dev_token\" (
    token_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES \"user\"(user_id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scope TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_revoked BOOLEAN NOT NULL DEFAULT false,
    last_used_at TIMESTAMPTZ,
    CONSTRAINT token_name_length CHECK (LENGTH(name) >= 1 AND LENGTH(name) <= 255),
    CONSTRAINT valid_token_expiry CHECK (expires_at > created_at)
);

CREATE TABLE IF NOT EXISTS \"post\" (
    post_id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES \"user\"(user_id) ON DELETE CASCADE,
    is_tagged BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_username ON \"user\"(username);
CREATE INDEX IF NOT EXISTS idx_user_is_active ON \"user\"(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_session_user_id ON \"session\"(user_id);
CREATE INDEX IF NOT EXISTS idx_session_is_revoked ON \"session\"(is_revoked) WHERE is_revoked = false;
CREATE INDEX IF NOT EXISTS idx_dev_token_user_id ON \"dev_token\"(user_id);
CREATE INDEX IF NOT EXISTS idx_dev_token_is_active ON \"dev_token\"(is_revoked) WHERE is_revoked = false;
CREATE INDEX IF NOT EXISTS idx_post_user_id ON \"post\"(user_id);
CREATE INDEX IF NOT EXISTS idx_post_created_at ON \"post\"(created_at DESC);
";

/// Initialize database tables and collections
pub async fn database(pool: &Pool) -> Result<()> {
    //mongo initialization
    print!("===mongo initialization===");
    let mongo_client = match utility::connect_to_mongo().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("MongoDB connection failed: {}", e);
            return Err(e);
        }
    };
    let db = mongo_client.database("image");
    let mongo_db_list = db.list_collection_names().await;
    if mongo_db_list.is_err() {
        eprintln!("Error while listing collection names");
        let err_msg = mongo_db_list.as_ref().err().unwrap().to_string();
        eprintln!("{}", err_msg);
        return Err(Error::new(ErrorKind::Other, err_msg));
    }
    for collection_name in mongo_db_list.unwrap() {
        println!("Collection name: {}", collection_name);
    }
    //postgres initialization
    println!("===postgres initialization===");
    let psql_client = match pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to get connection from pool: {}", e);
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
    };
    //create tables
    match psql_client.batch_execute(POSTGRES_SQL).await {
        Ok(_) => println!("Postgres tables initialized successfully"),
        Err(e) => {
            eprintln!("Error while initializing Postgres tables");
            eprintln!("{}", e.to_string());
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
    }
    println!("===Finish Initialization===");
    Ok(())
}
