use crate::types::{ErrorResponse, ItemResponse, ResponseFile, UploadJson};
use crate::utility::get_psql_pool;
use crate::{DESTINATION, MONGODB_DBANAME};
use actix_files::NamedFile;
use actix_web::{HttpResponse, Responder, web};
use deadpool_postgres::Pool;
use mongodb::Client;
use mongodb::bson::{Binary, doc, spec::BinarySubtype};
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

pub async fn get_one(
    psql_pool: web::Data<Pool>,
    mongo_pool: web::Data<Client>,
    item_id: web::Path<String>,
) -> io::Result<impl Responder> {
    let post_id_uuid = match Uuid::parse_str(&item_id.into_inner()) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                error: "Invalid post ID format".to_string(),
            }));
        }
    };
    let clinet = match get_psql_pool(&psql_pool).await {
        Ok(conn) => conn,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to get database connection".to_string(),
            }));
        }
    };
    let (post_id, filename, _content_type) = match clinet
        .query_one(
            "SELECT post_id,filename,content_type FROM post WHERE post_id = $1",
            &[&post_id_uuid],
        )
        .await
    {
        Ok(row) => {
            let post_id = row.get::<_, Uuid>(0);
            let filename = row.get::<_, String>(1);
            let _content_type = row.get::<_, String>(2);
            (post_id, filename, _content_type)
        }
        Err(e) => {
            eprintln!("Query Error : {}", e);
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                error: "Item not found".to_string(),
            }));
        }
    };
    let coll = mongo_pool
        .database(MONGODB_DBANAME)
        //we do not use Post type here so that rust fails convert type(mongo express uuid as bin)
        .collection::<mongodb::bson::Document>("post");
    let uuid_binary = Binary {
        subtype: BinarySubtype::Generic,
        bytes: post_id.as_bytes().to_vec(),
    };
    let filter = doc! {"post_id": uuid_binary};
    let doc_result = match coll.find_one(filter).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("MongoDB query error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Database query failed".to_string(),
            }));
        }
    };

    let (meta_title, meta_creator, meta_source, meta_description) = match doc_result {
        Some(doc) => {
            let title = doc.get_str("title").unwrap_or("").to_string();
            let creator = doc.get_str("creator").unwrap_or("").to_string();
            let source = doc.get_str("source").unwrap_or("").to_string();
            let description = doc.get_str("description").unwrap_or("").to_string();
            (title, creator, source, description)
        }
        None => {
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                error: "the content you are looking for is not found.".to_string(),
            }));
        }
    };
    Ok(HttpResponse::Ok().json(ItemResponse {
        image: filename,
        metadata: UploadJson {
            title: meta_title,
            creator: meta_creator,
            source: meta_source,
            description: meta_description,
        },
    }))
}
pub async fn open_file(item: web::Path<String>) -> io::Result<impl Responder> {
    let filename = item.into_inner();
    if filename.contains("..") || filename.starts_with("/") || filename.starts_with("\\") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Invalid file path",
        ));
    }
    let base_path = PathBuf::from(DESTINATION);
    let full_path = base_path.join(&filename);
    match full_path.canonicalize() {
        Ok(canonical_path) => {
            let canonical_base = match base_path.canonicalize() {
                Ok(path) => path,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "Base directory not found",
                    ));
                }
            };
            if !canonical_path.starts_with(&canonical_base) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Access denied: path outside allowed directory",
                ));
            }
            NamedFile::open(&canonical_path)
        }
        Err(e) => {
            eprintln!("File not found or access denied: {}", e);
            Err(e)
        }
    }
}
pub async fn get_all(pool: web::Data<Pool>) -> io::Result<impl Responder> {
    let client = match pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to get connection from pool: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to get database connection".to_string(),
            }));
        }
    };
    let result = client.query("SELECT post_id FROM post", &[]).await;
    let ids: Vec<Uuid> = match result {
        Ok(row_vec) => row_vec
            .iter()
            .map(|f| -> Uuid { f.get::<_, Uuid>("post_id") })
            .collect(),
        Err(e) => {
            eprintln!("Query failed: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Query failed".to_string(),
            }));
        }
    };
    let response = ResponseFile {
        file: ids.iter().map(|id| -> String { id.to_string() }).collect(),
    };
    Ok(HttpResponse::Ok().json(response))
}
