use actix_cors::Cors;
use actix_multipart::form::MultipartFormConfig;
use actix_web::{
    App, HttpResponse, HttpServer, Responder,
    http::{StatusCode, header::ContentType},
    web,
};
use mediapub::{
    ACTIX_PORT, ACTIX_SERVER, MAX_PAYLOAD_SIZE, db_pool::create_pool, init, route::{items::{get_all, get_one}, ping::ping, upload::upload}
};
use std::io::{self, Error};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    //create pool
    let pool = match create_pool() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create database pool: {}", e);
            return Err(Error::new(
                io::ErrorKind::Other,
                "Failed to create database pool",
            ));
        }
    };
    //initialize database
    match init::database(&pool).await {
        Ok(_) => println!("Database initialized successfully"),
        Err(e) => {
            eprintln!("Database initialization failed: {}", e);
            return Err(Error::new(
                io::ErrorKind::Other,
                "Database initialization failed",
            ));
        }
    }
    let launch_msg = format!("Starting Server on {}:{}...",ACTIX_SERVER,ACTIX_PORT);

    println!("{}",&launch_msg);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::PayloadConfig::new(MAX_PAYLOAD_SIZE))
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(MAX_PAYLOAD_SIZE)
                    .memory_limit(MAX_PAYLOAD_SIZE),
            )
            .wrap(Cors::default().allow_any_origin().allow_any_method())
            .app_data(web::Data::new(pool.clone()))
            .service(web::resource("/ping").route(web::get().to(ping)))
            .service(
                web::resource("/upload")
                    .route(web::get().to(index))
                    .route(web::post().to(upload)),
            )
            .service(web::resource("/item").route(web::get().to(get_all)))
            .service(web::resource("/item/{item_id}").route(web::get().to(get_one)))
    })
    .bind((ACTIX_SERVER,ACTIX_PORT))?
    .workers(2)
    .run()
    .await
}

/// Simple HTML form for testing uploads
async fn index() -> io::Result<impl Responder> {
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
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::html())
        .body(html))
}
