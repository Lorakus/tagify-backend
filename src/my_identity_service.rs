#![allow(dead_code)]

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use futures::future::{ok, FutureExt, LocalBoxFuture, Ready};

use actix_http::{Response, ResponseBuilder};
use actix_web::dev::{Extensions, Payload, ServiceRequest, ServiceResponse};
use actix_web::error::{Error, Result};
use actix_web::http::StatusCode;
use actix_web::{FromRequest, HttpMessage, HttpRequest, HttpResponse};
use log::{debug, error};

use deadpool_postgres::Pool;

use crate::db::get_user_by_name;
use crate::errors::HandlerError;
use crate::models::User;
use crate::my_cookie_policy::MyCookieIdentityPolicy;

#[derive(Clone)]
pub struct Identity(HttpRequest);

pub async fn login_user(
    req: HttpRequest,
    cookie_factory: &MyCookieIdentityPolicy,
    user: User,
) -> Response {
    let mut resp = ServiceResponse::new(req, HttpResponse::new(StatusCode::OK));
    if let Some(id) = resp.request().extensions_mut().get_mut::<IdentityItem>() {
        id.user = Some(user.clone());
        id.changed = true;
    }

    let cookie_name = user.role.clone();

    match cookie_factory
        .to_response(Some(user), true, &cookie_name, &mut resp)
        .await
    {
        Ok(_) => (),
        Err(e) => error!("Could not set cookie {}", e),
    }
    let login = resp.response();

    let mut resp = ResponseBuilder::new(StatusCode::OK);

    for c in login.cookies() {
        resp.cookie(c);
    }
    resp.finish()
}

impl Identity {
    /// Return the claimed identity of the user associated request or
    /// ``None`` if no identity can be found associated with the request.
    pub fn identity(&self) -> User {
        Identity::get_identity(&self.0.extensions())
    }

    /// This method is used to 'forget' the current identity on subsequent
    /// requests.
    pub fn logout(&self) {
        if let Some(id) = self.0.extensions_mut().get_mut::<IdentityItem>() {
            id.changed = true;
            id.user = None;
        }
    }

    fn get_identity(extensions: &Extensions) -> User {
        if let Some(id) = extensions.get::<IdentityItem>() {
            id.user.as_ref().unwrap().clone()
        } else {
            panic!("user is None, this should not happen");
        }
    }
}

#[derive(Debug, Clone)]
struct IdentityItem {
    user: Option<User>,
    changed: bool,
}

/// Helper trait that allows to get Identity.
///
/// It could be used in middleware but identity policy must be set before any other middleware that needs identity
/// RequestIdentity is implemented both for `ServiceRequest` and `HttpRequest`.
pub trait RequestIdentity {
    fn get_identity(&self) -> User;
}

impl<T> RequestIdentity for T
where
    T: HttpMessage,
{
    fn get_identity(&self) -> User {
        Identity::get_identity(&self.extensions())
    }
}

impl FromRequest for Identity {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Identity, Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Identity(req.clone()))
    }
}

/// Identity policy definition.
pub trait IdentityPolicy: Sized + 'static {
    /// The return type of the middleware
    type Future: Future<Output = Result<Option<String>, Error>>;

    /// The return type of the middleware
    type ResponseFuture: Future<Output = Result<(), Error>>;

    /// Parse the session from request and load data from a service identity.
    fn from_request(&self, request: &mut ServiceRequest) -> Self::Future;

    /// Write changes to response
    fn to_response<B>(
        &self,
        user: Option<User>,
        changed: bool,
        cookie_name: &str,
        response: &mut ServiceResponse<B>,
    ) -> Self::ResponseFuture;
}

pub struct IdentityService<T> {
    backend: Rc<T>,
    pool: Pool,
}

impl<T> IdentityService<T> {
    /// Create new identity service with specified backend.
    pub fn new(backend: T, s_pool: Pool) -> Self {
        IdentityService {
            backend: Rc::new(backend),
            pool: s_pool,
        }
    }
}

impl<S, T, B> Transform<S> for IdentityService<T>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    T: IdentityPolicy,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = IdentityServiceMiddleware<S, T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(IdentityServiceMiddleware {
            backend: self.backend.clone(),
            service: Rc::new(RefCell::new(service)),
            pool: self.pool.clone(),
        })
    }
}

#[doc(hidden)]
pub struct IdentityServiceMiddleware<S, T> {
    backend: Rc<T>,
    service: Rc<RefCell<S>>,
    pool: Pool,
}

impl<S, T> Clone for IdentityServiceMiddleware<S, T> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            service: self.service.clone(),
            pool: self.pool.clone(),
        }
    }
}

impl<S, T, B> Service for IdentityServiceMiddleware<S, T>
where
    B: 'static,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    T: IdentityPolicy,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let srv = self.service.clone();
        let backend = self.backend.clone();
        let fut = self.backend.from_request(&mut req);
        let pool = self.pool.clone();

        async move {
            let client = match pool.get().await {
                Ok(item) => item,
                Err(e) => {
                    error!("Failed to get client from pool: {}", e);
                    return Ok(req.error_response(HandlerError::InternalError));
                }
            };

            match fut.await {
                Ok(maybe_id) => {
                    let id = match maybe_id {
                        Some(id) => id,
                        None => {
                            error!("Could not extract id from request");
                            return Ok(req.error_response(HandlerError::AuthFail));
                        }
                    };

                    debug!("Username in cookie is {}", id);
                    let user: User = match get_user_by_name(client, &id).await {
                        Ok(user) => user,
                        Err(e) => {
                            error!("get_user failed {}", e);
                            return Ok(req.error_response(HandlerError::AuthFail));
                        }
                    };

                    debug!("Extracted user is: {:?}", user);
                    let cookie_name = user.role.clone();

                    req.extensions_mut().insert(IdentityItem {
                        user: Some(user),
                        changed: false,
                    });

                    // https://github.com/actix/actix-web/issues/1263
                    let fut = { srv.borrow_mut().call(req) };
                    let mut res = match fut.await {
                        Ok(i) => i,
                        Err(e) => {
                            error!("call failed: {}", e);
                            panic!("Help");
                        }
                    };
                    let id = res.request().extensions_mut().remove::<IdentityItem>();

                    if let Some(id) = id {
                        match backend
                            .to_response(id.user, id.changed, &cookie_name, &mut res)
                            .await
                        {
                            Ok(_) => Ok(res),
                            Err(e) => {
                                error!("to_response failed: {}", e);
                                Ok(res.error_response(e))
                            }
                        }
                    } else {
                        Ok(res)
                    }
                }
                Err(err) => {
                    error!("from_request failed: {}", err);
                    Ok(req.error_response(err))
                }
            }
        }
        .boxed_local()
    }
}
