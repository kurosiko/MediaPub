use std::io::{self, Error};

use actix_cors::Cors;
use actix_files::NamedFile;
use actix_multipart::form::{MultipartForm, MultipartFormConfig, tempfile::TempFile};
use actix_web::{App, HttpResponse, HttpServer, Responder, dev::Path, get, http::{StatusCode, header::ContentType}, web};
use rusqlite::Connection;
use uuid::Uuid;
use serde::Serialize;

const MAX_PAYLOAD_SIZE:usize = 1024 * 1024 * 1024;
const DB_PATH:&str = "./data/database.db";
#[actix_web::main]
async fn  main() -> io::Result<()>{
    //database initialization
    let connection = match Connection::open(DB_PATH) {
        Ok(conn) => conn,
        Err(e)=>{
            println!("Error while establish connection to db");
            println!("{}",e.to_string());
            return Err(Error::new(io::ErrorKind::Other,e.to_string()))
            
        }
    };
    match connection.execute(
        "CREATE TABLE IF NOT EXISTS item (
            id TEXT PRIMARY KEY,
            file TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    ) {
        Ok(_) => println!("table created or already exists"),
        Err(e) => {
            println!("Failed to create table: {}", e);
            return Err(Error::new(io::ErrorKind::Other, e.to_string()))
        }
    };
    HttpServer::new(move ||{
        App::new()
        .app_data(web::PayloadConfig::new(MAX_PAYLOAD_SIZE))
        .app_data(MultipartFormConfig::default().total_limit(MAX_PAYLOAD_SIZE).memory_limit(MAX_PAYLOAD_SIZE))
        .wrap(
            Cors::default()
            .allow_any_origin()
            .allow_any_method()
        )
        .service(ping)
        .service(
            web::resource("/upload")
            .route(web::get().to(index))
            .route(web::post().to(upload))
        )
        .service(
            web::resource("/get")
            .route(web::get().to(get))
        )
        .service(
            web::resource("/item/{item_id}")
            .route(web::get().to(get_item))
        )

    })
    .bind(("0.0.0.0",8080))?
    .workers(2)
    .run()
    .await
}

#[get("/ping")]
async fn ping() -> io::Result<impl Responder>{
    Ok(HttpResponse::build(StatusCode::OK).content_type(ContentType::plaintext()).body("hello world!"))
}

async fn get_item(item_id:web::Path<String>)-> io::Result<impl Responder>{
    println!("item_id is {}",item_id);
    Ok(NamedFile::open(format!("{}/{}",DESTINATION,item_id)))
}


#[derive(Debug,MultipartForm)]
struct UploadForm{
    #[multipart(limit= "500MB")]
    file:Vec<TempFile>
}


async fn get() -> io::Result<impl Responder>{
    let connection = match Connection::open(DB_PATH) {
        Ok(conn)=>conn,
        Err(e)=>{
            println!("Error while establish connection to db");
            println!("{}",e.to_string());
            return Err(Error::new(io::ErrorKind::Other,e.to_string()));
        }
    };
    let mut stmt = match connection.prepare("SELECT file FROM item") {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to prepare statement: {}", e);
            return Err(Error::new(io::ErrorKind::Other, e.to_string()));
        }
    };
    let files_iter = match stmt.query_map([], |row| row.get::<_, String>(0)) {
        Ok(iter) => iter,
        Err(e) => {
            println!("Failed to query: {}", e);
            return Err(Error::new(io::ErrorKind::Other, e.to_string()));
        }
    };
    let mut files = Vec::new();
    for file in files_iter {
        match file {
            Ok(f) => files.push(f),
            Err(e) => {
                println!("Failed to get file: {}", e);
                return Err(Error::new(io::ErrorKind::Other, e.to_string()));
            }
        }
    }
    #[derive(Serialize)]
    struct ResponseJson{
        files:Vec<String>
    }
    let body_json = ResponseJson { files:files };
    Ok(HttpResponse::Ok().content_type(ContentType::json()).json(body_json))
}

const DESTINATION:&str = "./tmp";
async fn upload(MultipartForm(form):MultipartForm<UploadForm>) -> io::Result<impl Responder> {
    let connection = match Connection::open(DB_PATH) {
        Ok(conn) => conn,
        Err(e)=>{
            println!("Error while establish connection to db");
            println!("{}",e.to_string());
            return Err(Error::new(io::ErrorKind::Other,e.to_string()))
            
        }
    };
    for f in form.file.into_iter(){
        match f.content_type {
            Some(ct_type)=>{
                println!("Content_Type is {}",ct_type.essence_str())
            },
            None=>{
                println!("Content_Type is none")
            }
        }
        println!("Size:{}",f.size);
        let filename = match f.file_name {
            Some(name) => name,
            None => {
                println!("Name: none");
                return Ok(HttpResponse::BadRequest().body("extention was not found."));
            }
        };
        println!("Name:{}", filename);
        let ext = filename.rsplit('.').next();
        if ext == None {return Ok(HttpResponse::BadRequest().body("extention was not found."))}
        let ext = ext.unwrap();
        let new_filename = format!("{}.{}",Uuid::new_v4(),ext);
        let path = format!("{}/{}",DESTINATION,new_filename);
        match f.file.persist(&path) {
            Ok(_)=>println!("{} saved successfully",filename),
            Err(_)=>println!("{} failed to save",filename)
        };
        let _ = match connection.execute("
            INSERT INTO item (id, file)
            VALUES (?1, ?2)
        ",(&Uuid::new_v4().to_string().as_str(),&new_filename)
        ) {
            Ok(_) => {},
            Err(e) => {
                println!("Insert Error {}",e.to_string());
                return Ok(HttpResponse::InternalServerError().finish());
            }
        };
    };
    let html = r#"
    <html>
        <head>
            <title?>Thx for uploading!</title?
        </head>
        <body>
            <h1>THX</h1>
        </body>
    </html>
    "#;
    
    Ok(HttpResponse::build(StatusCode::OK).content_type(ContentType::html()).body(html))
}

async fn index()-> io::Result<impl Responder>{
    let html = r#"
    <html>
        <head>
            <title>uploader</title>
        </head>
        <body>
            <form action="/upload" method="post" enctype="multipart/form-data">
                <input type="file" name="file"/>
                <button type="submit">Submit</button>
            </form>
        </body>
       
        
    </html>
    "#;
    Ok(HttpResponse::build(StatusCode::OK).content_type(ContentType::html()).body(html))
}