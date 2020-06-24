use crate::errors::HandlerError;
use crate::models::{CreateUser, UpdateUserAdmin, User};
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Result};
use deadpool_postgres::Pool;
use log::error;

use crate::db;

pub async fn create_user(
    pool: web::Data<Pool>,
    data: web::Json<CreateUser>,
) -> Result<HttpResponse, HandlerError> {
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured : {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let result = match db::create_user(&client, &data).await {
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

pub async fn update_user(
    pool: web::Data<Pool>,
    id: web::Path<(i32,)>,
    data: web::Json<UpdateUserAdmin>,
) -> Result<HttpResponse, HandlerError> {
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured : {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let user = match db::get_user(&client, id.0).await {
        Ok(i) => i,
        Err(e) => {
            error!("Error occured : {}", e);
            return Err(HandlerError::BadClientData {
                field: "User id does not exist".to_owned(),
            });
        }
    };

    let new_user = User {
        nickname: data.nickname.clone(),
        password: data.password.clone(),
        role: data.role.clone(),
        ..user
    };

    let result = match db::update_user(&client, &new_user).await {
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(item) => item,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

pub async fn delete_user(
    pool: web::Data<Pool>,
    data: web::Path<(i32,)>,
) -> Result<HttpResponse, HandlerError> {
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let result = db::delete_user(&client, data.0).await;

    match result {
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(_res) => {}
    };

    Ok(HttpResponse::new(StatusCode::OK))
}
