use actix_multipart::form::{MultipartForm, json::Json, tempfile::TempFile};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Post {
    pub post_id: Uuid,
    pub title: String,
    pub creator: String,
    pub source: String,
    pub description: String,
    pub uploader: Uuid,
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFile {
    pub file: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadJson {
    pub title: String,
    pub creator: String,
    pub source: String,
    pub description: String,
}

#[derive(Debug, MultipartForm)]
pub struct UploadFrom {
    #[multipart(limit = "10MB")]
    pub file: Vec<TempFile>,
    pub metadata: Json<Vec<UploadJson>>,
}

#[derive(Debug, Deserialize)]
pub struct SignUpRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignUpResponse {
    pub user_id: String,
    pub username: String,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Deserialize)]
pub struct LoginSession {
    pub session_token: String,
}
#[derive(Debug, Deserialize)]
pub struct RefreshToken {
    pub refresh_token: String,
}
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub username: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SessionTokenResponse {
    pub user_id: String,
    pub username: String,
    pub session_token: String,
    pub refresh_token: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct ItemResponse {
    pub image: String,
    pub metadata: UploadJson,
}
