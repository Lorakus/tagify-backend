use actix_web::{http::header, HttpResponse, ResponseError};
use failure::Fail;

use actix_http::ResponseBuilder;
use actix_web::http::StatusCode;

/*
 * Only to be used in admin_handlers.rs & handlers.rs
 */
#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "Parsing error on field: {}", field)]
    BadClientData { field: String },
    #[fail(display = "An internal error occured. Try again later")]
    InternalError,
    #[fail(display = "You are not logged in")]
    AuthFail,
}

impl ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            HandlerError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::BadClientData { .. } => StatusCode::BAD_REQUEST,
            HandlerError::AuthFail => StatusCode::UNAUTHORIZED,
        }
    }
}

/*
 * Only to be used in db.rs
 */
#[derive(Fail, Debug)]
pub enum DBError {
    #[fail(display = "Query to struct mapper error")]
    MapperError(tokio_pg_mapper::Error),

    #[fail(display = "Postgres error")]
    PostgresError(tokio_postgres::Error),

    #[fail(display = "Argon error")]
    ArgonError(argon2::Error),

    #[fail(display = "Bad argument: {}", err)]
    BadArgs { err: String },
}
impl From<argon2::Error> for DBError {
    fn from(err: argon2::Error) -> DBError {
        DBError::ArgonError(err)
    }
}
impl From<tokio_postgres::Error> for DBError {
    fn from(err: tokio_postgres::Error) -> DBError {
        DBError::PostgresError(err)
    }
}
impl From<tokio_pg_mapper::Error> for DBError {
    fn from(err: tokio_pg_mapper::Error) -> DBError {
        DBError::MapperError(err)
    }
}
