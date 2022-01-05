use actix_files as fs;
use actix_web::{
    middleware,
    web::{self, Data},
    App, HttpRequest, HttpServer, Responder,
};
use anyhow::Result;
use dotenv::dotenv;
use sqlx::MySqlPool;
use std::env;
mod handlers;

#[macro_use]
extern crate log;

async fn search(req: HttpRequest) -> impl Responder {
    let params = req.match_info().get("name").unwrap_or("World!");
    format!("You searched for {}", params)
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE is not set");
    let host = env::var("HOST").expect("HOST ip is not set");
    let port = env::var("PORT")
        .expect("PORT is not set")
        .parse::<u16>()
        .expect("Port must be a number");
    let db_pool = MySqlPool::connect(&database_url).await?;
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db_pool.clone()))
            .wrap(middleware::Logger::default())
            .route("/search", web::get().to(search))
            .route("/search/{name}", web::get().to(search))
            .route("/signup", web::post().to(handlers::user::signup))
            .route("/login", web::post().to(handlers::user::login))
            .route("/user/{id}", web::post().to(handlers::user::get))
            .route("/home", web::route())
            .service(fs::Files::new("/css", "./css/").show_files_listing())
            .default_service(
                fs::Files::new("/home", "./static/")
                    .index_file("index.html")
                    .show_files_listing(),
            )
    })
    .bind((host, port))?
    .run()
    .await?;
    Ok(())
}
