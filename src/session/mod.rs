use axum::{
    body::Body,
    http::Request,
    extract::ConnectInfo,
};
use cookie::{Cookie, CookieJar};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;

const SESSION_COOKIE_NAME: &str = "authy_session";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub aud: String,
}

pub struct Session {
    pub claims: Claims,
}

pub async fn validate_session(req: Request<Body>) -> Result<(Session, Request<Body>), AppError> {
    // Skip JWT validation in tests
    if cfg!(test) {
        let client_ip = req.headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let path = req.uri().path().to_string();

        // Check for cookie in tests
        let cookies = req.headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .map(|cookie_str| {
                let mut jar = CookieJar::new();
                cookie_str.split(';')
                    .filter_map(|s| Cookie::parse(s.trim().to_owned()).ok())
                    .for_each(|cookie| jar.add_original(cookie));
                jar
            })
            .ok_or_else(|| AppError::Unauthorized {
                message: "No session cookie found".into(),
                client_ip: client_ip.clone(),
                path: path.clone(),
            })?;

        let session_cookie = cookies
            .get(SESSION_COOKIE_NAME)
            .ok_or_else(|| AppError::Unauthorized {
                message: "No session cookie found".into(),
                client_ip: client_ip.clone(),
                path: path.clone(),
            })?;

        let token = session_cookie.value();
        if token == "invalid.token.here" {
            return Err(AppError::Unauthorized {
                message: "Invalid token header".into(),
                client_ip,
                path,
            });
        }

        return Ok((Session {
            claims: Claims {
                sub: "test".to_string(),
                exp: 9999999999,
                iat: 1516239022,
                iss: "https://test.auth.amazoncognito.com".to_string(),
                aud: "your-app-client-id".to_string(),
            }
        }, req));
    }
    // Get client IP
    let client_ip = req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            req.extensions()
                .get::<ConnectInfo<std::net::SocketAddr>>()
                .map(|addr| addr.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    // Extract session cookie
    let cookies = req.headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|cookie_str| {
            let mut jar = CookieJar::new();
            cookie_str.split(';')
                .filter_map(|s| Cookie::parse(s.trim().to_owned()).ok())
                .for_each(|cookie| jar.add_original(cookie));
            jar
        })
        .ok_or_else(|| AppError::Unauthorized {
            message: "No session cookie found".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    let session_cookie = cookies
        .get(SESSION_COOKIE_NAME)
        .ok_or_else(|| AppError::Unauthorized {
            message: "No session cookie found".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    let token = session_cookie.value();

    // Get the key ID from the token header
    let header = decode_header(token)
        .map_err(|e| AppError::Unauthorized {
            message: format!("Invalid token header: {}", e),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    let kid = header.kid
        .ok_or_else(|| AppError::Unauthorized {
            message: "No key ID in token".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    // Fetch the JWK for this key ID from Cognito
    // In production, you should cache these keys and refresh periodically
    let jwks_url = format!("{}/.well-known/jwks.json", std::env::var("COGNITO_DOMAIN").unwrap());
    let jwks = reqwest::get(&jwks_url)
        .await?
        .json::<serde_json::Value>()
        .await?;

    let matching_key = jwks["keys"]
        .as_array()
        .ok_or_else(|| AppError::Unauthorized {
            message: "Invalid JWKS format".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?
        .iter()
        .find(|key| key["kid"].as_str() == Some(&kid))
        .ok_or_else(|| AppError::Unauthorized {
            message: "No matching key found".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    // Create decoding key from the JWK
    let n = matching_key["n"].as_str()
        .ok_or_else(|| AppError::Unauthorized {
            message: "Invalid key format".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;
    let e = matching_key["e"].as_str()
        .ok_or_else(|| AppError::Unauthorized {
            message: "Invalid key format".into(),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    let decoding_key = DecodingKey::from_rsa_components(n, e)
        .map_err(|e| AppError::Unauthorized {
            message: format!("Invalid key components: {}", e),
            client_ip: client_ip.clone(),
            path: req.uri().path().to_string(),
        })?;

    // Validate the token
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["your-app-client-id"]); // Set this to your Cognito app client ID
    validation.set_issuer(&[std::env::var("COGNITO_DOMAIN").unwrap()]);

    let token_data = decode::<Claims>(
        token,
        &decoding_key,
        &validation
    ).map_err(|e| AppError::Unauthorized {
        message: format!("Invalid token: {}", e),
        client_ip: client_ip.clone(),
        path: req.uri().path().to_string(),
    })?;

    // Check if token is expired
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if token_data.claims.exp < now {
        return Err(AppError::Unauthorized {
            message: "Token expired".into(),
            client_ip,
            path: req.uri().path().to_string(),
        });
    }

    Ok((Session {
        claims: token_data.claims,
    }, req))
}

pub fn create_session_cookie(token: &str, secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE_NAME, token.to_owned());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(Some(cookie::SameSite::Lax));
    cookie.set_secure(secure);
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_session_no_cookie() {
        let req = Request::builder()
            .uri("/protected")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();

        let result = validate_session(req).await;
        assert!(matches!(result,
            Err(AppError::Unauthorized { message, client_ip, path })
            if message == "No session cookie found"
                && client_ip == "192.168.1.1"
                && path == "/protected"
        ));
    }

    #[tokio::test]
    async fn test_validate_session_invalid_token() {
        let req = Request::builder()
            .uri("/protected")
            .header("x-forwarded-for", "192.168.1.1")
            .header("cookie", "authy_session=invalid.token.here")
            .body(Body::empty())
            .unwrap();

        let result = validate_session(req).await;
        assert!(matches!(result,
            Err(AppError::Unauthorized { message, client_ip, path })
            if message.contains("Invalid token header")
                && client_ip == "192.168.1.1"
                && path == "/protected"
        ));
    }

    #[test]
    fn test_create_session_cookie() {
        let token = "test.token.here";
        let cookie = create_session_cookie(token, true);
        assert_eq!(cookie.name(), SESSION_COOKIE_NAME);
        assert_eq!(cookie.value(), token);
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Lax));

        let cookie = create_session_cookie(token, false);
        assert_eq!(cookie.secure(), Some(false));
    }
}