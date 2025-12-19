use crate::{MONGODB_URI, };
use actix_web::HttpResponse;
use deadpool_postgres::{Object, Pool};
use serde::Deserialize;
use uuid::Uuid;

pub async fn connect_to_mongo() -> std::io::Result<mongodb::Client> {
    match mongodb::Client::with_uri_str(MONGODB_URI).await {
        Ok(client) => Ok(client),
        Err(e) => {
            eprintln!("Failed to connect to MongoDB: {}", e);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        }
    }
}

pub async fn get_psql_pool(pool:&Pool) -> std::io::Result<Object>{
    match pool.get().await {
        Ok(conn) => Ok(conn),
        Err(e)=>{
            eprintln!("Failed to get connection from pool: {}", e);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get database connection",
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum CredentialType {
    SessionToken,
    DevToken,
}

use crate::errors::{
    AHError::{AccountSuspended, InvalidCredential, UserInactive},
    DBError::{ConnectionFailed, QueryFailed},
    DBType::Postgres,
    ErrorKind::{self, AuthError, DatabaseError},
};

fn extract_user_id_from_row(row: &tokio_postgres::Row) -> Result<Uuid, ErrorKind> {
    let user_id_bytes: Vec<u8> = row.get(0);
    match Uuid::from_slice(&user_id_bytes) {
        Ok(uuid) => Ok(uuid),
        Err(e) => {
            eprintln!("Invalid user_id format: {}", e);
            Err(DatabaseError(QueryFailed(Postgres)))
        }
    }
}

/// Check user validity using a connection from the pool
/// This is the preferred method to use in request handlers
pub async fn check_user_validity_with_pool(
    pool: &Pool,
    credential: &str,
    credential_type: CredentialType,
) -> Result<Uuid, ErrorKind> {
    let psql_client = match pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to get connection from pool: {}", e);
            return Err(DatabaseError(ConnectionFailed(Postgres)));
        }
    };

    let user_id = match credential_type {
        CredentialType::DevToken => {
            match psql_client 
                .query_one(
                    "SELECT user_id FROM dev_token WHERE token_hash = $1 AND is_revoked = false",
                    &[&credential],
                )
                .await
            {
                Ok(row) => extract_user_id_from_row(&row),
                Err(e) => {
                    eprintln!("Dev token query failed: {}", e);
                    Err(DatabaseError(QueryFailed(Postgres)))
                }
            }
        }
        CredentialType::SessionToken => {
            match psql_client 
                .query_one(
                    "SELECT user_id FROM session WHERE session_token = $1 AND is_revoked = false",
                    &[&credential],
                ).await
            {
                Ok(row) => extract_user_id_from_row(&row),
                Err(e) => {
                    eprintln!("Session token query failed: {}", e);
                    Err(DatabaseError(QueryFailed(Postgres)))
                }
            }
        }
    };
    let user_id = match user_id {
        Ok(id) => id,
        Err(e) => return Err(e),
    };
    match psql_client .query_one(
        "SELECT is_active FROM \"user\" WHERE user_id = $1 AND is_active = true",
        &[&user_id.to_string()]
        ).await
    {
        Ok(_) => Ok(user_id),
        Err(e) => {
            eprintln!("User active check failed: {}", e);
            Err(AuthError(AccountSuspended))
        }
    }
}

pub fn generate_response(error: &ErrorKind) -> HttpResponse {
    match error {
        ErrorKind::AuthError(InvalidCredential) => {
            HttpResponse::Unauthorized().body("invalid credential")
        }
        ErrorKind::AuthError(UserInactive) => HttpResponse::Unauthorized().body("user inactive"),
        ErrorKind::AuthError(AccountSuspended) => {
            HttpResponse::Unauthorized().body("account suspended")
        }
        ErrorKind::DatabaseError(_) => HttpResponse::ExpectationFailed().finish(),
    }
}
