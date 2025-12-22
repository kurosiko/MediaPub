use crate::types::ErrorResponse;
use actix_web::HttpResponse;
use deadpool_postgres::{Object, Pool};
use serde::Deserialize;
use uuid::Uuid;


pub async fn get_psql_pool(pool: &Pool) -> std::io::Result<Object> {
    match pool.get().await {
        Ok(conn) => Ok(conn),
        Err(e) => {
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
    match row.try_get::<_, Uuid>(0) {
        Ok(uuid) => Ok(uuid),
        Err(e) => {
            eprintln!("Invalid user_id format: {}", e);
            Err(DatabaseError(QueryFailed(Postgres)))
        }
    }
}

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
                )
                .await
            {
                Ok(row) => {
                    let user_id: Uuid = row.get(0);
                    Ok(user_id)
                }
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
    match psql_client
        .query_one(
            "SELECT is_active FROM \"user\" WHERE user_id = $1 AND is_active = true",
            &[&user_id],
        )
        .await
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
            HttpResponse::Unauthorized().json(ErrorResponse {
                error: "invalid credential".to_string(),
            })
        }
        ErrorKind::AuthError(UserInactive) => HttpResponse::Unauthorized().json(ErrorResponse {
            error: "user inactive".to_string(),
        }),
        ErrorKind::AuthError(AccountSuspended) => {
            HttpResponse::Unauthorized().json(ErrorResponse {
                error: "account suspended".to_string(),
            })
        }
        ErrorKind::DatabaseError(_) => HttpResponse::ExpectationFailed().json(ErrorResponse {
            error: "Database error".to_string(),
        }),
    }
}
