use crate::{
    types::{ErrorResponse, SignUpRequest, SignUpResponse},
    utility::get_psql_pool,
};
use actix_web::{HttpResponse, Responder, web};
use bcrypt::{DEFAULT_COST, hash};
use deadpool_postgres::Pool;
use uuid::Uuid;

pub async fn signup(
    pool: web::Data<Pool>,
    data: web::Json<SignUpRequest>,
) -> std::io::Result<impl Responder> {
    if data.username.trim().is_empty() || data.password.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Username and password cannot be empty".to_string(),
        }));
    }
    if data.username.len() < 3 || data.username.len() > 255 {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Username must be between 3 and 255 characters".to_string(),
        }));
    }
    if data.password.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "Password must be at least 8 characters long".to_string(),
        }));
    }

    let psql_client = match get_psql_pool(&pool).await {
        Ok(client) => client,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Database connection error".to_string(),
            }));
        }
    };

    let password_hash = match hash(&data.password, DEFAULT_COST) {
        Ok(hashed_pass) => hashed_pass,
        Err(e) => {
            eprintln!("Failed to hash password: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to process password".to_string(),
            }));
        }
    };

    let user_id = Uuid::new_v4();
    let query = r#"
        INSERT INTO "user" (user_id, username, password_hash)
        VALUES ($1, $2, $3)
        RETURNING user_id, username
    "#;

    match psql_client
        .query_one(query, &[&user_id, &data.username, &password_hash])
        .await
    {
        Ok(row) => {
            let returned_user_id: Uuid = row.get(0);
            let returned_username: String = row.get(1);
            Ok(HttpResponse::Created().json(SignUpResponse {
                user_id: returned_user_id.to_string(),
                username: returned_username,
                message: "User registered successfully".to_string(),
            }))
        }
        Err(e) => {
            match psql_client
                .query_one(
                    "SELECT EXISTS(SELECT 1 FROM \"user\" WHERE username = $1)",
                    &[&data.username],
                )
                .await
            {
                Ok(is_exist) => {
                    let exists: bool = is_exist.get(0);
                    if exists {
                        return Ok(HttpResponse::Conflict().json(ErrorResponse {
                            error: "Username already taken".to_string(),
                        }));
                    }
                }
                Err(_) => {
                    return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Database query error".to_string(),
                    }));
                }
            }
            eprintln!("Failed to insert user: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create user".to_string(),
            }))
        }
    }
}
