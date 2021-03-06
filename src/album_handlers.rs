
use crate::album_models::{CreateAlbum, AlbumsPreview, UpdateAlbum, TagPhoto, VerifyPhoto};
use crate::user_models::{User};

use crate::errors::{HandlerError, DBError};
use crate::my_identity_service::Identity;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Result};
use deadpool_postgres::Pool;
use log::{error, info};

use crate::db;

pub async fn create_album(
    pool: web::Data<Pool>,
    data: web::Json<CreateAlbum>,
    id: Identity,
    tagify_albums_path: web::Data<String,>,
) -> Result<HttpResponse, HandlerError> {
    let user: User = id.identity();
    let first_photo = String::from("default_path");

    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured : {}", e);
            return Err(HandlerError::InternalError);
        }
    };
    //create album without tags
    let result = match db::create_album(&client, &data, user.id, first_photo).await {
        Err(e) => {
            error!("Error occured after create_album: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(album) => {
            //TODO create album folder on photo_server
            let path = format!("{}{}", tagify_albums_path.to_string(), &album.id);
            match std::fs::create_dir_all(&path) {
                Ok(_) => info!("Created folder for album with id={}", &album.id),
                Err(e) => {
                    error!("Error creating folder for album with id={}: {:?}", &album.id, e);
                    return Err(HandlerError::InternalError);
                }
            }
            album
        },
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

pub async fn get_own_albums(
    pool: web::Data<Pool>,
    id: Identity,
) -> Result<HttpResponse, HandlerError> {
    let user: User = id.identity();

    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let result = match db::get_users_albums(&client, user.id).await {
        Err(e) => {
            error!("Error occured get users albums: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

pub async fn get_album_by_id(
    pool: web::Data<Pool>,
    album_id: web::Path<(i32,)>,
) -> Result<HttpResponse, HandlerError> {
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    // TODO ERROR album with this id does not exists
    let result = match db::get_album_by_id(&client, album_id.0).await {
        Err(e) => {
            error!("Error occured get users albums: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}



// gets all albums data (id, title, description, first_photo)
pub async fn get_all_albums(
    pool: web::Data<Pool>
) -> Result<HttpResponse, HandlerError> {
    
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let albums: AlbumsPreview = match db::get_all_albums(client).await {
        Ok(albums) => albums,
        Err(e) => match e {
            DBError::PostgresError(e) => {
                error!("Getting albums failed {}", e);
                return Err(HandlerError::AuthFail);
            }
            DBError::MapperError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            DBError::ArgonError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            DBError::BadArgs { err } => {
                error!("Error occured: {}", err);
                return Err(HandlerError::BadClientData {
                    field: err.to_owned(),
                });
            }
        },
    };
    Ok(HttpResponse::build(StatusCode::OK).json(albums)) 
}



// get 20 next photos from album (start at 20 * index)
pub async fn get_photos_from_album(
    pool: web::Data<Pool>,
    data : web::Path<(i32, i32)>
) -> Result<HttpResponse, HandlerError> {
  
  let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    // TODO catch error if album with id does not exists
  let result = match db::get_photos_from_album(client, &data.0, &data.1).await {
        Err(e) => {
            error!("Error occured : {}", e);
              return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

   Ok(HttpResponse::build(StatusCode::OK).json(result)) 
    
}


  pub async fn delete_album_by_id(
    pool: web::Data<Pool>,
    album_id: web::Path<(i32,)>,
    id: Identity,
) -> Result<HttpResponse, HandlerError> {
    let user: User = id.identity();
  
    let client = match pool.get().await {
          Ok(item) => item,
          Err(e) => {
              error!("Error occured: {}", e);
              return Err(HandlerError::InternalError);
          }
      };

    let result = match db::get_album_by_id(&client, album_id.0).await {
        Err(e) => {
            error!("Error occured get users albums: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

    if user.id == result.users_id || user.role == "admin" {
        println!("usunie album");
        match db::delete_album(&client, album_id.0).await {
            Err(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            Ok(result) => result,
        };
    } else {
        //TODO ERROR you are not owner of this album
    }
    Ok(HttpResponse::new(StatusCode::OK))
}



  
pub async fn update_album_by_id(
    pool: web::Data<Pool>,
    album_id: web::Path<(i32,)>,
    id: Identity,
    data: web::Json<UpdateAlbum>,
) -> Result<HttpResponse, HandlerError> {
    let user: User = id.identity();
  
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };
  
    let result = match db::get_album_by_id(&client, album_id.0).await {
        Err(e) => {
            error!("Error occured get users albums: {}", e);

            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };


    if user.id == result.users_id || user.role == "admin" {
        match db::update_album(&client, album_id.0, &data).await {
            Err(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            Ok(num_updated) => num_updated,
        };
    } else {
        //TODO ERROR you are not owner of this album
    }
    Ok(HttpResponse::new(StatusCode::OK))
}

// tag photo + set coordinates
pub async fn tag_photo_by_id(
    pool: web::Data<Pool>,
    data_id : web::Path<(i32,)>,
    data: web::Json<TagPhoto>,
) -> Result<HttpResponse, HandlerError> {
  
  let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    
     match db::tag_photo_by_id(client, &data_id.0, &data).await {
        Err(e) => {
            error!("Error occured : {}", e);
              return Err(HandlerError::InternalError);
        }
        Ok(item) => match item {
            true => return Ok(HttpResponse::build(StatusCode::OK).json(item)),
            false => {
                error!("Error occured : timeout");
                return Err(HandlerError::BadClientData {
                    field: "timeout".to_string()
                });
            }
        }
    };
 
}

// verify_photo
pub async fn verify_photo_by_id(
    pool: web::Data<Pool>,
    data_id : web::Path<(i32,)>,
    data: web::Json<VerifyPhoto>,
) -> Result<HttpResponse, HandlerError> {
  
  let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

  
     match db::verify_photo_by_id(client, &data_id.0, data.verified).await {
        Err(e) => {
            error!("Error occured : {}", e);
              return Err(HandlerError::InternalError);
        }
        Ok(item) => match item {
            true => return Ok(HttpResponse::build(StatusCode::OK).json(item)),
            false => {
                error!("Error occured : timeout");
                return Err(HandlerError::BadClientData {
                    field: "timeout".to_string()
                });
            }
        }
    };

}

// get next 20 photos for tagging 
pub async fn get_photos_for_tagging(
    pool: web::Data<Pool>,
    data : web::Path<(i32, )>
) -> Result<HttpResponse, HandlerError> {
  
  let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    
    
  let result = match db::get_photos_for_tagging(client, &data.0).await {
        Err(e) => {
            error!("Error occured : {}", e);
              return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

   Ok(HttpResponse::build(StatusCode::OK).json(result)) 
  
    
}