use crate::CLIENT as client;
use anyhow::Context;
use common::errors::ApiError;
use reqwest::header::AUTHORIZATION;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt;

fn url(route: impl Into<String>) -> String {
    let base = yew::utils::window().location().origin().unwrap();
    format!("{}{}", base, route.into())
}
fn to_sentence_case(input: &str) -> String {
    input
        .split('.')
        .map(|it| {
            let it = it.trim();
            if !it.is_ascii() || it.is_empty() {
                return it.to_string();
            }
            let (head, tail) = it.split_at(1);
            head.to_uppercase() + tail + "."
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[derive(Debug, Clone)]
pub struct NoContent;

impl fmt::Display for NoContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "server returned 204")
    }
}

pub async fn request<T: Serialize, R: DeserializeOwned>(
    request_url: impl Into<String>,
    method: Method,
    body: Option<&T>,
    auth_token: Option<&str>,
) -> anyhow::Result<R> {
    let mut builder = match method {
        Method::POST => client
            .post(&url(request_url))
            .body(serde_json::to_string(body.as_ref().unwrap())?),
        Method::GET => client.get(&url(request_url)),
        _ => unreachable!(),
    };

    if let Some(token) = auth_token {
        if !token.is_empty() {
            builder = builder.header(AUTHORIZATION, token);
        }
    }

    let resp = builder.send().await?;
    let status = resp.status();
    if status.is_success() {
        let res = resp.json::<R>().await;
        if status == StatusCode::NO_CONTENT {
            res.context(NoContent)
        } else {
            Ok(res?)
        }
    } else {
        let error = resp.json::<ApiError>().await?;
        Err(anyhow::anyhow!("{}", to_sentence_case(&error.message)))
    }
}

#[macro_export]
macro_rules! request {
    (method = $method:ident, url = $url:expr) => {
        crate::services::request::request(
            $url,
            ::reqwest::Method::$method,
            Option::None,
            Option::None,
        )
    };
    (method = $method:ident, url = $url:expr, body = $body:expr) => {
        crate::services::request::request(
            $url,
            ::reqwest::Method::$method,
            Option::Some($body),
            Option::None,
        )
    };

    (method = $method:ident, url = $url:expr, token = $token:expr) => {
        crate::services::request::request(
            $url,
            ::reqwest::Method::$method,
            Option::Some(&"".to_string()),
            Option::Some($token),
        )
    };
    (method = $method:ident, url = $url:expr, body = $body:expr, token = $token:expr) => {
        crate::services::request::request(
            $url,
            ::reqwest::Method::$method,
            Option::Some($body),
            Option::Some($token),
        )
    };
}
