#![cfg_attr(feature = "strict", deny(warnings))]

use actix_files as fs;
use actix_files::NamedFile;
use actix_web::{middleware::Logger, web, App, HttpServer, Result};
use std::path::PathBuf;

use listenfd::ListenFd;
use log::{error, info};
use std::fs::File;
use std::io::Read;
use tokio_postgres::NoTls;

mod config;
mod db;
mod errors;
mod handlers;

mod admin_handlers;
mod album_handlers;
mod my_cookie_policy;
mod my_identity_service;

mod album_models;
mod user_models;

use crate::handlers::{login, logout, status};
use user_models::ROLES;

struct DistPath {
    path: PathBuf,
}

async fn index(data: web::Data<DistPath>) -> Result<NamedFile> {
    Ok(NamedFile::open(data.path.clone())?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    if cfg!(debug_assertions) {
        // Setup logging
        std::env::set_var("RUST_LOG", "DEBUG");
        std::env::set_var("RUST_BACKTRACE", "1");
    } else {
        std::env::set_var("RUST_LOG", "INFO");
    }
    // Initialize logger
    env_logger::init();

    let mut conf_path = PathBuf::new();

    // If debug build, use execution directory
    if cfg!(debug_assertions) {
        conf_path.push(".");
    } else {
        // Check if CONFIG_DIR environment variable is available
        let conf_base =
            std::env::var("CONFIG_DIR").expect("Could not find environment variable CONFIG_DIR");
        conf_path.push(conf_base);
        if !conf_path.exists() {
            panic!(
                "CONFIG_DIR env variable does not point to a valid directory: {}",
                conf_path.to_str().unwrap()
            );
        }
    }
    info!("CONFIG_DIR points to: {}", conf_path.to_str().unwrap());

    // Read config
    let settings_path = conf_path.join("Settings");
    let conf = match crate::config::MyConfig::new(settings_path.to_str().unwrap()) {
        Ok(i) => i,
        Err(e) => {
            error!(
                "Could not read Settings file at {} err: {}",
                settings_path.to_str().unwrap(),
                e
            );
            panic!("Could not read Settings file");
        }
    };

    // Create db connection pool
    let pool = conf.postgres.create_pool(NoTls).unwrap();

    // Create connection to database
    let client = match pool.get().await {
        Ok(i) => i,
        Err(e) => {
            error!("Could not connect to database err: {}", e);
            panic!("Could not connect to database");
        }
    };

    // Read schema.sql and create db table
    let mut schema = String::new();
    let schema_path = conf_path.join("schema.sql");
    match File::open(schema_path.clone()) {
        Ok(mut i) => i.read_to_string(&mut schema).unwrap(),
        Err(e) => {
            error!(
                "Could not open schema.sql file at {} err: {}",
                schema_path.to_str().unwrap(),
                e
            );
            panic!("Could not open schema.sql file");
        }
    };

    match client.batch_execute(&schema).await {
        Ok(i) => i,
        Err(e) => {
            error!("Failed to apply db schema err: {}", e);
            panic!("Failed to apply db schema");
        }
    }

    // Build server address
    let ip = conf.server.hostname + ":" + &conf.server.port;
    println!("Server is reachable at http://{}", ip);



     // Create default admin accounts
     match db::create_user(&client, &conf.default_admin).await {
         Ok(_item) => info!("Created default admin account"),
         Err(_e) => info!("Default user already exists"),
     }

    // // Create default user accounts
     match db::create_user(&client, &conf.default_user).await {
         Ok(_item) => info!("Created default user"),
         Err(_e) => info!("Default user already exists"),
     }

    let temp = conf.server.key.clone();

    // Register http routes
    let mut server = HttpServer::new(move || {
        let serve_file_service: fs::Files;
        let path_arg: DistPath;
        let secure_cookie: bool;
        let max_age = 30 * 24 * 60 * 60; // days

        // Check if in release mode if so use DIST env variable as path for serving frontend
        if cfg!(debug_assertions) {
            // If debug binary then hardcode path
            serve_file_service =
                fs::Files::new("/app/frontend/debug_dist", "../frontend/debug_dist")
                    .show_files_listing();
            path_arg = DistPath {
                path: PathBuf::from("../frontend/debug_dist/index.html"),
            };
            secure_cookie = false;
        } else {
            // If release binary use DIST env var
            let dist = std::env::var("DIST").expect("Could not find environment variable DIST");
            path_arg = DistPath {
                path: PathBuf::from(dist.clone()).join("index.html"),
            };
            if !std::path::Path::new(&dist).exists() {
                panic!(
                    "DIST env variable does not point to a valid directory: {}",
                    dist
                );
            }
            serve_file_service = fs::Files::new("/app/frontend/dist", dist).show_files_listing();
            secure_cookie = true;
        }
        let cookie_key = temp.as_bytes();

        let cookie_factory_user = my_cookie_policy::MyCookieIdentityPolicy::new(cookie_key)
            .name(ROLES[1])
            .path("/")
            .secure(secure_cookie)
            .max_age(max_age)
            .same_site(actix_http::cookie::SameSite::Strict);

        let cookie_factory_admin = my_cookie_policy::MyCookieIdentityPolicy::new(cookie_key)
            .name(ROLES[0])
            .path("/")
            .secure(secure_cookie)
            .max_age(max_age)
            .same_site(actix_http::cookie::SameSite::Strict);
        App::new()
            .data(path_arg)
            // Give login handler access to cookie factory
            .data(cookie_factory_user.clone())
            // Serve every file in directory from ../dist
            .service(serve_file_service)
            // Serve index.html
            .route("/", web::get().to(index))
            // Give every handler access to the db connection pool
            .data(pool.clone())
            // Enable logger
            .wrap(Logger::default())
            //limit the maximum amount of data that server will accept
            .data(web::JsonConfig::default().limit(4096)) // max 4MB json
            // .configure(routes)
            //status
            .service(
                web::scope("/api")
                    //all admin endpoints
                    .service(web::resource("/status").route(web::get().to(status)))
                    .service(web::resource("/login").route(web::post().to(login)))
                    .service(
                        web::scope("/admin")
                            .wrap(my_identity_service::IdentityService::new(
                                cookie_factory_admin,
                                pool.clone(),
                            ))
                            .route("/logout", web::post().to(logout))
                            //get all users
                            .route("/users", web::get().to(admin_handlers::get_all_users))
                            //create new user account
                            .route("/users", web::post().to(admin_handlers::create_user))
                            //get user by id
                            .route("/user/{user_id}", web::get().to(status))
                            //change user password
                            .route(
                                "/user/{user_id}",
                                web::put().to(admin_handlers::update_user),
                            )
                            // delete user account
                            .route(
                                "/user/{user_id}",
                                web::delete().to(admin_handlers::delete_user),
                            )
                            .service(
                                web::scope("/albums")
                                    //get all albums
                                    .route("", web::get().to(status))
                                    //change album data (description or name)
                                    .route(
                                        "/{album_id}",
                                        web::put().to(album_handlers::update_album_by_id),
                                    )
                                    //delete  album by id
                                    .route(
                                        "/{album_id}",
                                        web::delete().to(album_handlers::delete_album_by_id),
                                    )
                                    //delete photo from album
                                    .route(
                                        "/{album_id}/photos/{photo_id}",
                                        web::delete().to(status),
                                    ),
                            ), //.route("/user/{id}", web::get().to(admin_handlers::get_user))
                    )
                    //user auth routes
                    .service(
                        web::scope("/user")
                            .wrap(my_identity_service::IdentityService::new(
                                cookie_factory_user,
                                pool.clone(),
                            ))
                            .route("/logout", web::post().to(logout))
                            .route("/me", web::get().to(handlers::get_user))
                            .route("/me", web::delete().to(handlers::delete_user))
                            //update only nickname
                            .route("/me", web::put().to(handlers::update_user))
                            //update password
                            .route("/me/password", web::put().to(status))
                            .service(
                                web::scope("/albums")
                                    //get all own albums
                                    .route("", web::get().to(album_handlers::get_own_albums))
                                    //create new album
                                    .route("", web::post().to(album_handlers::create_album))
                                    //change album data (description or name)
                                    .route(
                                        "/{album_id}",
                                        web::put().to(album_handlers::update_album_by_id),
                                    )
                                    //add photos to album
                                    .route("/{album_id}", web::post().to(status))
                                    //delete own album
                                    .route(
                                        "/{album_id}",
                                        web::delete().to(album_handlers::delete_album_by_id),
                                    )
                                    //delete own album
                                    .route(
                                        "/{album_id}/photos/{photo_id}",
                                        web::delete().to(status),
                                    ),
                            )
                            .service(
                                web::scope("/tag")
                                    //get 15 photos for tagging
                                    .route("", web::get().to(status))
                                    //get own album by id
                                    .route("/{album_id}/action/{photo_id}", web::post().to(status))
                                    //verify tag
                                    .route("/{album_id}/action/{photo_id}", web::put().to(status)),
                            ),
                    )
                    .service(
                        web::scope("/albums")
                            //get albums for preview (all)
                            .route("", web::get().to(album_handlers::get_all_albums))
                            //get album by id
                            .route(
                                "/{album_id}{_:/?}",
                                web::get().to(album_handlers::get_album_by_id),
                            )
                            //get photos from album (preview)
                            .route("/{album_id}/photos/{index}", web::get().to(album_handlers::get_photos_from_album)),
                    ),
            )
    });

    // Enables us to hot reload the server
    let mut listenfd = ListenFd::from_env();
    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind(ip)?
    };

    server.run().await
}
