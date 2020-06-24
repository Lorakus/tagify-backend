#![allow(dead_code)]

use crate::my_identity_service::IdentityPolicy;
use actix_web::cookie::{Cookie, CookieJar, Key, SameSite};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::{Error, Result};
use actix_web::HttpMessage;
use futures::future::{ok, Ready};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::time::SystemTime;
use time::Duration;

use crate::models::User;

struct MyCookieIdentityInner {
    key: Key,
    key_v2: Key,
    name: String,
    path: String,
    domain: Option<String>,
    secure: bool,
    max_age: Option<Duration>,
    same_site: Option<SameSite>,
    visit_deadline: Option<Duration>,
    login_deadline: Option<Duration>,
}

#[derive(Deserialize, Serialize, Debug)]
struct CookieValue {
    identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    login_timestamp: Option<SystemTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visit_timestamp: Option<SystemTime>,
}

#[derive(Debug)]
struct CookieIdentityExtention {
    login_timestamp: Option<SystemTime>,
}

#[derive(Clone)]
pub struct MyCookieIdentityPolicy(Rc<MyCookieIdentityInner>);

impl MyCookieIdentityInner {
    fn new(key: &[u8]) -> MyCookieIdentityInner {
        let key_v2: Vec<u8> = key.iter().chain([1, 0, 0, 0].iter()).cloned().collect();
        MyCookieIdentityInner {
            key: Key::from_master(key),
            key_v2: Key::from_master(&key_v2),
            name: "actix-identity".to_owned(),
            path: "/".to_owned(),
            domain: None,
            secure: true,
            max_age: None,
            same_site: None,
            visit_deadline: None,
            login_deadline: None,
        }
    }

    fn set_cookie<B>(
        &self,
        resp: &mut ServiceResponse<B>,
        value: Option<CookieValue>,
        cookie_name: &str,
    ) -> Result<()> {
        let add_cookie = value.is_some();
        let val = value.map(|val| {
            if !self.legacy_supported() {
                serde_json::to_string(&val)
            } else {
                Ok(val.identity)
            }
        });
        let mut cookie = Cookie::new(
            cookie_name.to_owned(),
            val.unwrap_or_else(|| Ok(String::new()))?,
        );
        cookie.set_path(self.path.clone());
        cookie.set_secure(self.secure);
        cookie.set_http_only(true);

        if let Some(ref domain) = self.domain {
            cookie.set_domain(domain.clone());
        }

        if let Some(max_age) = self.max_age {
            cookie.set_max_age(max_age);
        }

        if let Some(same_site) = self.same_site {
            cookie.set_same_site(same_site);
        }

        let mut jar = CookieJar::new();
        let key = if self.legacy_supported() {
            &self.key
        } else {
            &self.key_v2
        };
        if !add_cookie {
            let mut now = time::now();
            now.tm_year -= 999; // TODO: don't hardcode this
            cookie.set_expires(now);
        }
        jar.private(&key).add(cookie);
        for cookie in jar.delta() {
            resp.response_mut()
                .add_cookie(cookie)
                .expect("Identity could not set cookie");
        }
        Ok(())
    }

    fn load(&self, req: &ServiceRequest) -> Option<CookieValue> {
        let cookie = req.cookie(&self.name)?;
        let mut jar = CookieJar::new();
        jar.add_original(cookie.clone());
        let res = if self.legacy_supported() {
            jar.private(&self.key).get(&self.name).map(|n| CookieValue {
                identity: n.value().to_string(),
                login_timestamp: None,
                visit_timestamp: None,
            })
        } else {
            None
        };
        res.or_else(|| {
            jar.private(&self.key_v2)
                .get(&self.name)
                .and_then(|c| self.parse(c))
        })
    }

    fn parse(&self, cookie: Cookie) -> Option<CookieValue> {
        let value: CookieValue = serde_json::from_str(cookie.value()).ok()?;
        let now = SystemTime::now();
        if let Some(visit_deadline) = self.visit_deadline {
            if now.duration_since(value.visit_timestamp?).ok()? > visit_deadline.to_std().ok()? {
                return None;
            }
        }
        if let Some(login_deadline) = self.login_deadline {
            if now.duration_since(value.login_timestamp?).ok()? > login_deadline.to_std().ok()? {
                return None;
            }
        }
        Some(value)
    }

    fn legacy_supported(&self) -> bool {
        self.visit_deadline.is_none() && self.login_deadline.is_none()
    }

    fn always_update_cookie(&self) -> bool {
        self.visit_deadline.is_some()
    }

    fn requires_oob_data(&self) -> bool {
        self.login_deadline.is_some()
    }
}

impl MyCookieIdentityPolicy {
    /// Construct new `MyCookieIdentityPolicy` instance.
    ///
    /// Panics if key length is less than 32 bytes.
    pub fn new(key: &[u8]) -> MyCookieIdentityPolicy {
        MyCookieIdentityPolicy(Rc::new(MyCookieIdentityInner::new(key)))
    }

    /// Sets the `path` field in the session cookie being built.
    pub fn path<S: Into<String>>(mut self, value: S) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().path = value.into();
        self
    }

    /// Sets the `name` field in the session cookie being built.
    pub fn name<S: Into<String>>(mut self, value: S) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().name = value.into();
        self
    }

    /// Sets the `domain` field in the session cookie being built.
    pub fn domain<S: Into<String>>(mut self, value: S) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().domain = Some(value.into());
        self
    }

    /// Sets the `secure` field in the session cookie being built.
    ///
    /// If the `secure` field is set, a cookie will only be transmitted when the
    /// connection is secure - i.e. `https`
    pub fn secure(mut self, value: bool) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().secure = value;
        self
    }

    /// Sets the `max-age` field in the session cookie being built with given number of seconds.
    pub fn max_age(self, seconds: i64) -> MyCookieIdentityPolicy {
        self.max_age_time(Duration::seconds(seconds))
    }

    /// Sets the `max-age` field in the session cookie being built with `chrono::Duration`.
    pub fn max_age_time(mut self, value: Duration) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().max_age = Some(value);
        self
    }

    /// Sets the `same_site` field in the session cookie being built.
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        Rc::get_mut(&mut self.0).unwrap().same_site = Some(same_site);
        self
    }

    /// Accepts only users whose cookie has been seen before the given deadline
    ///
    /// By default visit deadline is disabled.
    pub fn visit_deadline(mut self, value: Duration) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().visit_deadline = Some(value);
        self
    }

    /// Accepts only users which has been authenticated before the given deadline
    ///
    /// By default login deadline is disabled.
    pub fn login_deadline(mut self, value: Duration) -> MyCookieIdentityPolicy {
        Rc::get_mut(&mut self.0).unwrap().login_deadline = Some(value);
        self
    }
}

impl IdentityPolicy for MyCookieIdentityPolicy {
    type Future = Ready<Result<Option<String>, Error>>;
    type ResponseFuture = Ready<Result<(), Error>>;

    fn from_request(&self, req: &mut ServiceRequest) -> Self::Future {
        ok(self.0.load(req).map(
            |CookieValue {
                 identity,
                 login_timestamp,
                 ..
             }| {
                if self.0.requires_oob_data() {
                    req.extensions_mut()
                        .insert(CookieIdentityExtention { login_timestamp });
                }
                identity
            },
        ))
    }

    fn to_response<B>(
        &self,
        id: Option<User>,
        changed: bool,
        cookie_name: &str,
        res: &mut ServiceResponse<B>,
    ) -> Self::ResponseFuture {
        let _ = if changed {
            let login_timestamp = SystemTime::now();
            self.0.set_cookie(
                res,
                id.map(|identity| CookieValue {
                    identity: identity.username,
                    login_timestamp: self.0.login_deadline.map(|_| login_timestamp),
                    visit_timestamp: self.0.visit_deadline.map(|_| login_timestamp),
                }),
                cookie_name,
            )
        } else if self.0.always_update_cookie() && id.is_some() {
            let visit_timestamp = SystemTime::now();
            let login_timestamp = if self.0.requires_oob_data() {
                let CookieIdentityExtention {
                    login_timestamp: lt,
                } = res.request().extensions_mut().remove().unwrap();
                lt
            } else {
                None
            };
            self.0.set_cookie(
                res,
                Some(CookieValue {
                    identity: id.unwrap().username,
                    login_timestamp,
                    visit_timestamp: self.0.visit_deadline.map(|_| visit_timestamp),
                }),
                cookie_name,
            )
        } else {
            Ok(())
        };
        ok(())
    }
}
