use crate::client::Client as UserClient;
use anyhow::Result;
use reqwest::{Client as ReqwestClient, Response};

pub struct Httpc;

impl Httpc {
    fn attach_auth_info<T>(
        builder: reqwest::RequestBuilder,
        client: &UserClient<T>,
    ) -> reqwest::RequestBuilder {
        if let Some(token) = client.auth_token.as_ref() {
            builder.header("Authorization", token.as_str())
        } else {
            builder
        }
    }

    pub async fn get<T>(
        client: &UserClient<T>,
        url: &str,
        query_params: Option<Vec<(&str, &str)>>,
    ) -> Result<Response> {
        let http = ReqwestClient::new();
        let mut request = http.get(url);
        request = Self::attach_auth_info(request, client);

        if let Some(pairs) = query_params {
            request = request.query(&pairs);
        }

        let resp = request.send().await?;
        Ok(resp)
    }

    pub async fn post<T>(
        client: &UserClient<T>,
        url: &str,
        body_content: String,
    ) -> Result<Response> {
        let http = ReqwestClient::new();
        let mut request = http.post(url).header("Content-Type", "application/json");
        request = Self::attach_auth_info(request, client);
        let resp = request.body(body_content).send().await?;
        Ok(resp)
    }

    pub async fn delete<T>(client: &UserClient<T>, url: &str) -> Result<Response> {
        let http = ReqwestClient::new();
        let request = http.delete(url);
        let request = Self::attach_auth_info(request, client);
        let resp = request.send().await?;
        Ok(resp)
    }

    pub async fn patch<T>(
        client: &UserClient<T>,
        url: &str,
        body_content: String,
    ) -> Result<Response> {
        let http = ReqwestClient::new();
        let mut request = http.patch(url).header("Content-Type", "application/json");
        request = Self::attach_auth_info(request, client);
        let resp = request.body(body_content).send().await?;
        Ok(resp)
    }
}
