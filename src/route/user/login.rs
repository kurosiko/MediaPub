use actix_web::{HttpResponse, Responder, web};
use bcrypt::verify;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Pool;
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    types::{ErrorResponse, LoginRequest, LoginResponse, RefreshToken, SessionTokenResponse},
    utility::get_psql_pool,
};

pub async fn raw(
    pool: web::Data<Pool>,
    data: web::Json<LoginRequest>,
) -> std::io::Result<impl Responder> {
    if data.username.trim().is_empty() || data.password.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "username or password is invalid.".to_string(),
        }));
    }

    let psql_client = match get_psql_pool(&pool).await {
        Ok(client) => client,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database connection error.".to_string(),
            }));
        }
    };

    let user_row = match psql_client
        .query_one(
            "SELECT user_id, password_hash, username FROM \"user\" WHERE username = $1",
            &[&data.username],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                error: "username or password is invalid.".to_string(),
            }));
        }
    };

    let user_id: Uuid = user_row.get(0);
    let password_hash: String = user_row.get(1);
    let username: String = user_row.get(2);

    match verify(&data.password, &password_hash) {
        Ok(is_valid) => {
            if !is_valid {
                return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                    error: "username or password is invalid.".to_string(),
                }));
            }
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "password verification failed.".to_string(),
            }));
        }
    }

    match generate_session_tokens(&psql_client, &user_id, &username).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse { error: e })),
    }
}

pub async fn session_token_login(
    pool: web::Data<Pool>,
    data: web::Json<crate::types::LoginSession>,
) -> std::io::Result<impl Responder> {
    if data.session_token.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "session token is invalid.".to_string(),
        }));
    }

    let psql_client = match get_psql_pool(&pool).await {
        Ok(client) => client,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database connection error.".to_string(),
            }));
        }
    };

    let query = "SELECT user_id FROM session WHERE session_token = $1 AND is_revoked = false";
    let user_id: Uuid = match psql_client.query_one(query, &[&data.session_token]).await {
        Ok(row) => row.get(0),
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                error: "invalid session token.".to_string(),
            }));
        }
    };

    let username: String = match psql_client
        .query_one(
            "SELECT username FROM \"user\" WHERE user_id = $1",
            &[&user_id],
        )
        .await
    {
        Ok(row) => row.get(0),
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database query error.".to_string(),
            }));
        }
    };

    Ok(HttpResponse::Ok().json(LoginResponse {
        user_id: user_id.to_string(),
        username,
        message: "login successfully.".to_string(),
    }))
}

pub async fn refresh_token(
    pool: web::Data<Pool>,
    data: web::Json<RefreshToken>,
) -> std::io::Result<impl Responder> {
    if data.refresh_token.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "refresh token is invalid.".to_string(),
        }));
    }

    let psql_client = match get_psql_pool(&pool).await {
        Ok(client) => client,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database connection error.".to_string(),
            }));
        }
    };

    let refresh_token_hash = hash_token(&data.refresh_token);

    let query = r#"
        SELECT user_id, refresh_expires_at FROM session
        WHERE refresh_token_hash = $1 AND is_revoked = false
    "#;

    let session_row = match psql_client.query_one(query, &[&refresh_token_hash]).await {
        Ok(row) => row,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
                error: "invalid refresh token.".to_string(),
            }));
        }
    };

    let user_id: Uuid = session_row.get(0);
    let refresh_expires_at: DateTime<Utc> = session_row.get(1);

    if refresh_expires_at < Utc::now() {
        return Ok(HttpResponse::Unauthorized().json(ErrorResponse {
            error: "refresh token has expired.".to_string(),
        }));
    }

    let username: String = match psql_client
        .query_one(
            "SELECT username FROM \"user\" WHERE user_id = $1",
            &[&user_id],
        )
        .await
    {
        Ok(row) => row.get(0),
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "database query error.".to_string(),
            }));
        }
    };

    match generate_session_tokens(&psql_client, &user_id, &username).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse { error: e })),
    }
}

async fn generate_session_tokens(
    psql_client: &deadpool_postgres::Client,
    user_id: &Uuid,
    username: &str,
) -> Result<SessionTokenResponse, String> {
    let token_id = Uuid::new_v4();
    let session_token = generate_random_token();
    let refresh_token = generate_random_token();
    let refresh_token_hash = hash_token(&refresh_token);

    let now = Utc::now();
    let session_expires_at = now + Duration::hours(1);
    let refresh_expires_at = now + Duration::days(30);

    let insert_query = r#"
        INSERT INTO "session" (token_id, user_id, session_token, refresh_token_hash, session_expires_at, refresh_expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    match psql_client
        .execute(
            insert_query,
            &[
                &token_id,
                &user_id,
                &session_token,
                &refresh_token_hash,
                &session_expires_at,
                &refresh_expires_at,
            ],
        )
        .await
    {
        Ok(_) => Ok(SessionTokenResponse {
            user_id: user_id.to_string(),
            username: username.to_string(),
            session_token,
            refresh_token,
            message: "login successfully.".to_string(),
        }),
        Err(e) => {
            eprintln!("Failed to insert session: {}", e);
            Err("failed to create session.".to_string())
        }
    }
}

fn generate_random_token() -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen_range(0..256) as u8).collect();
    hex::encode(random_bytes)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
