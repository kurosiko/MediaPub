use crate::DESTINATION;
use crate::types::ResponseFile;
use actix_files::NamedFile;
use actix_web::{HttpResponse, Responder, web};
use deadpool_postgres::Pool;
use std::io;

pub async fn get_one(item_id: web::Path<String>) -> io::Result<impl Responder> {
    println!("item_id is {}", item_id);
    Ok(NamedFile::open(format!("{}/{}", DESTINATION, item_id)))
}

pub async fn get_all(pool: web::Data<Pool>) -> io::Result<impl Responder> {
    // Get connection from pool
    let client = match pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to get connection from pool: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get database connection",
            ));
        }
    };

    // Query all post IDs
    let result = client.query("SELECT post_id FROM post", &[]).await;
    let ids = match result {
        Ok(row_vec) => row_vec
            .iter()
            .map(|file| -> String { file.get::<_, String>(0) })
            .collect::<Vec<String>>(),
        Err(e) => {
            eprintln!("Query failed: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Query failed: {}", e),
            ));
        }
    };

    let response = ResponseFile { file: ids };
    Ok(HttpResponse::Ok().json(response))
}
