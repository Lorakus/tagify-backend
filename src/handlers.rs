use crate::errors::HandlerError;
use crate::models::{Hash, LoginData, SendUser, UpdateUser, User};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use deadpool_postgres::Pool;
use log::{debug, error};

use crate::db;
use crate::errors;
use crate::my_cookie_policy::MyCookieIdentityPolicy;
use crate::my_identity_service::{login_user, Identity};

pub async fn get_user(id: Identity) -> Result<HttpResponse, HandlerError> {
    // Get user identity
    let user: User = id.identity();

    let send_user = SendUser {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(send_user))
}

pub async fn logout(id: Identity) -> Result<HttpResponse, HandlerError> {
    id.logout();

    Ok(HttpResponse::new(StatusCode::OK))
}

pub async fn login(
    data: web::Json<LoginData>,
    pool: web::Data<Pool>,
    req: HttpRequest,
    cookie_factory: web::Data<MyCookieIdentityPolicy>,
) -> Result<HttpResponse, HandlerError> {
    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let user: User = match db::get_user_by_name(client, &data.username).await {
        Ok(user) => user,
        Err(e) => match e {
            errors::DBError::PostgresError(e) => {
                error!("Getting user failed: {}", e);
                return Err(HandlerError::AuthFail);
            }
            errors::DBError::MapperError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            errors::DBError::ArgonError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            errors::DBError::BadArgs { err } => {
                error!("Error occured: {}", err);
                return Err(HandlerError::BadClientData {
                    field: err.to_owned(),
                });
            }
        },
    };

    match user.verify_password(data.password.as_bytes()) {
        Ok(correct) => {
            if !correct {
                return Err(HandlerError::AuthFail);
            }
        }
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    }

    debug!("User {} logged in successfully", user.username);
    Ok(login_user(req, cookie_factory.get_ref(), user).await)
}

pub async fn update_user(
    pool: web::Data<Pool>,
    id: Identity,
    data: web::Json<UpdateUser>,
) -> Result<HttpResponse, HandlerError> {
    // Get user identity
    let user: User = id.identity();

    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let new_user = User {
        id: user.id,
        username: user.username,
        nickname: data.nickname.clone(),
        password: data.password.clone(),
        role: user.role,
    };

    let result = db::update_user(&client, &new_user).await;

    match result {
        Err(e) => match e {
            errors::DBError::PostgresError(e) => {
                error!("Getting user failed: {}", e);
                return Err(HandlerError::InternalError);
            }
            errors::DBError::MapperError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            errors::DBError::ArgonError(e) => {
                error!("Error occured: {}", e);
                return Err(HandlerError::InternalError);
            }
            errors::DBError::BadArgs { err } => {
                error!("Error occured: {}", err);
                return Err(HandlerError::BadClientData {
                    field: err.to_owned(),
                });
            }
        },
        Ok(num_updated) => num_updated,
    };

    Ok(HttpResponse::new(StatusCode::OK))
}

pub async fn delete_user(
    pool: web::Data<Pool>,
    id: Identity,
) -> Result<HttpResponse, HandlerError> {
    // Get user identity
    let user: User = id.identity();

    let client = match pool.get().await {
        Ok(item) => item,
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
    };

    let result = db::delete_user(&client, user.id).await;

    match result {
        Err(e) => {
            error!("Error occured: {}", e);
            return Err(HandlerError::InternalError);
        }
        Ok(num_updated) => num_updated,
    };

    Ok(HttpResponse::new(StatusCode::OK))
}
