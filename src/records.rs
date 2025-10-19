use crate::client::Client;
use crate::httpc::Httpc;
use crate::error::RecordViewError;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde::{de::DeserializeOwned, Deserialize};
use std::cmp;

#[derive(Debug, Clone)]
pub struct RecordsManager<'a, A> {
    pub client: &'a Client<A>,
    pub name: &'a str,
}

#[derive(Debug, Clone)]
pub struct RecordsListRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub collection_name: &'a str,
    pub filter: Option<String>,
    pub sort: Option<String>,
    pub expand: Option<String>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordList<T> {
    pub page: i32,
    pub per_page: i32,
    pub total_items: i32,
    pub items: Vec<T>,
}

impl<'a, A: Clone> RecordsListRequestBuilder<'a, A> {
    pub async fn call<T: Default + DeserializeOwned>(&self) -> Result<RecordList<T>> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.client.base_url, self.collection_name
        );

        let mut build_opts: Vec<(&str, &str)> = vec![];
        if let Some(filter_opts) = &self.filter {
            build_opts.push(("filter", filter_opts))
        }
        if let Some(sort_opts) = &self.sort {
            build_opts.push(("sort", sort_opts))
        }
        if let Some(expand_opts) = &self.expand {
            build_opts.push(("expand", expand_opts))
        }
        let per_page_opts = self.per_page.to_string();
        let page_opts = self.page.to_string();
        build_opts.push(("perPage", per_page_opts.as_str()));
        build_opts.push(("page", page_opts.as_str()));
        let resp = Httpc::get(self.client, &url, Some(build_opts))
            .await
            .with_context(|| format!("GET {} failed to execute", url))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .with_context(|| format!("Reading response body from {} failed", url))?;

        if !status.is_success() {
            let snippet_len = cmp::min(2000, body.len());
            let snippet = &body[..snippet_len];
            return Err(anyhow!(
                "Request to {} failed: HTTP {}.\nResponse (truncated):\n{}",
                url,
                status.as_u16(),
                snippet
            ));
        }

        let mut deserializer = serde_json::Deserializer::from_str(&body);
        match serde_path_to_error::deserialize::<_, RecordList<T>>(&mut deserializer) {
            Ok(parsed) => Ok(parsed),
            Err(de_err) => {
                let path = de_err.path().to_string();
                // Show a short snippet to help diagnose server-side data issues
                let snippet_len = cmp::min(2000, body.len());
                let snippet = &body[..snippet_len];

                Err(anyhow!(
                    "JSON decode error at path `{}`: {}\nResponse (truncated):\n{}",
                    path,
                    de_err,
                    snippet
                ))
            }
        }
    }

    pub async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        A: Clone,
        T: Default + DeserializeOwned,
    {
        let mut all_items = Vec::new();
        let mut page = 1;
        let per_page = 1000;
        let url = format!(
            "{}/api/collections/{}/records",
            self.client.base_url, self.collection_name
        );

        loop {
            let mut build_opts: Vec<(&str, &str)> = vec![];
            if let Some(filter_opts) = &self.filter {
                build_opts.push(("filter", filter_opts))
            }
            if let Some(sort_opts) = &self.sort {
                build_opts.push(("sort", sort_opts))
            }
            if let Some(expand_opts) = &self.expand {
                build_opts.push(("expand", expand_opts))
            }
            let per_page_opts = &per_page.to_string();
            let page_opts = &page.to_string();

            build_opts.push(("perPage", per_page_opts));
            build_opts.push(("page", page_opts));
            let result = Httpc::get(self.client, &url, Some(build_opts)).await;

            let page_resp = match result {
                Ok(result) => {
                    let response = result.json::<RecordList<T>>().await?;
                    Ok(response)
                }
                Err(e) => Err(e),
            }?;

            all_items.extend(page_resp.items.into_iter());

            if all_items.len() == page_resp.total_items as usize {
                break;
            }

            page += 1;
        }

        Ok(all_items)
    }

    pub fn filter(&self, filter_opts: &str) -> Self {
        Self {
            filter: Some(filter_opts.to_string()),
            ..self.clone()
        }
    }

    pub fn sort(&self, sort_opts: &str) -> Self {
        Self {
            sort: Some(sort_opts.to_string()),
            ..self.clone()
        }
    }

    pub fn expand(&self, expand_opts: &str) -> Self {
        Self {
            expand: Some(expand_opts.to_string()),
            ..self.clone()
        }
    }

    pub fn page(&self, page: i32) -> Self {
        Self {
            page,
            ..self.clone()
        }
    }

    pub fn per_page(&self, per_page: i32) -> Self {
        Self {
            per_page,
            ..self.clone()
        }
    }
}

pub struct RecordViewRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub collection_name: &'a str,
    pub identifier: &'a str,
}

impl<'a, A> RecordViewRequestBuilder<'a, A> {
    pub async fn call<T: Default + DeserializeOwned>(&self) -> Result<T, RecordViewError> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.client.base_url, self.collection_name, self.identifier
        );
        let resp = Httpc::get(self.client, &url, None)
            .await
            .with_context(|| format!("GET {} failed to execute", url))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .with_context(|| format!("Reading response body from {} failed", url))?;

        if !status.is_success() {
            let snippet_len = cmp::min(2000, body.len());
            let snippet = &body[..snippet_len];
            let code = status.as_u16();
            return if code == 404 {
                Err(RecordViewError::NotFound {
                    collection: self.collection_name.to_string(),
                    identifier: self.identifier.to_string(),
                    body_snippet: snippet.to_string(),
                })
            } else {
                Err(RecordViewError::Http {
                    status: code,
                    url,
                    body_snippet: snippet.to_string(),
                })
            };
        }

        let mut deserializer = serde_json::Deserializer::from_str(&body);
        match serde_path_to_error::deserialize::<_, T>(&mut deserializer) {
            Ok(parsed) => Ok(parsed),
            Err(de_err) => {
                // Show a short snippet to help diagnose server-side data issues
                let snippet_len = cmp::min(2000, body.len());
                let snippet = &body[..snippet_len];

                Err(RecordViewError::Decode {
                    path: de_err.path().to_string(),
                    source: de_err,
                    body_snippet: snippet.to_string(),
                })
            }
        }
    }
}

impl<'a, A> RecordDestroyRequestBuilder<'a, A> {
    pub async fn call(&self) -> Result<()> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.client.base_url, self.collection_name, self.identifier
        );
        match Httpc::delete(self.client, url.as_str()).await {
            Ok(result) => {
                if result.status() == 204 {
                    Ok(())
                } else {
                    Err(anyhow!("Failed to delete"))
                }
            }
            Err(e) => Err(anyhow!("error: {}", e)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordDestroyRequestBuilder<'a, A> {
    pub identifier: &'a str,
    pub client: &'a Client<A>,
    pub collection_name: &'a str,
}

#[derive(Debug, Clone)]
pub struct RecordDeleteAllRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub collection_name: &'a str,
    pub filter: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct RecordCreateRequestBuilder<'a, A, T: Serialize + Clone> {
    pub client: &'a Client<A>,
    pub collection_name: &'a str,
    pub record: T,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CreateResponse {
    #[serde(rename = "@collectionName")]
    pub collection_name: Option<String>,
    #[serde(rename = "@collectionId")]
    pub collection_id: Option<String>,
    pub id: String,
    pub updated: String,
    pub created: String,
}

impl<'a, A, T: Serialize + Clone> RecordCreateRequestBuilder<'a, A, T> {
    pub async fn call(&self) -> Result<CreateResponse> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.client.base_url, self.collection_name
        );
        let payload = serde_json::to_string(&self.record).map_err(anyhow::Error::from)?;
        match Httpc::post(self.client, &url, payload).await {
            Ok(result) => {
                let response = result.json::<CreateResponse>().await?;
                Ok(response)
            }
            Err(e) => Err(anyhow!("error: {}", e)),
        }
    }
}

pub struct RecordUpdateRequestBuilder<'a, A, T: Serialize + Clone> {
    pub record: T,
    pub collection_name: &'a str,
    pub client: &'a Client<A>,
    pub id: &'a str,
}

impl<'a, A, T: Serialize + Clone> RecordUpdateRequestBuilder<'a, A, T> {
    pub async fn call(&self) -> Result<T> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.client.base_url, self.collection_name, self.id
        );
        let payload = serde_json::to_string(&self.record).map_err(anyhow::Error::from)?;
        match Httpc::patch(self.client, &url, payload).await {
            Ok(result) => {
                result.json::<CreateResponse>().await?;
                Ok(self.record.clone())
            }
            Err(e) => Err(anyhow!("error: {}", e)),
        }
    }
}

impl<'a, A> RecordsManager<'a, A> {
    pub fn view(&self, identifier: &'a str) -> RecordViewRequestBuilder<'a, A> {
        RecordViewRequestBuilder {
            identifier,
            client: self.client,
            collection_name: self.name,
        }
    }

    pub fn destroy(&self, identifier: &'a str) -> RecordDestroyRequestBuilder<'a, A> {
        RecordDestroyRequestBuilder {
            identifier,
            client: self.client,
            collection_name: self.name,
        }
    }

    pub fn update<T: Serialize + Clone>(
        &self,
        identifier: &'a str,
        record: T,
    ) -> RecordUpdateRequestBuilder<'a, A, T> {
        RecordUpdateRequestBuilder {
            client: self.client,
            collection_name: self.name,
            id: identifier,
            record,
        }
    }

    pub fn create<T: Serialize + Clone>(&self, record: T) -> RecordCreateRequestBuilder<'a, A, T> {
        RecordCreateRequestBuilder {
            record,
            client: self.client,
            collection_name: self.name,
        }
    }

    pub fn list(&self) -> RecordsListRequestBuilder<'a, A> {
        RecordsListRequestBuilder {
            client: self.client,
            collection_name: self.name,
            filter: None,
            sort: None,
            expand: None,
            page: 1,
            per_page: 100,
        }
    }

    pub async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        A: Clone,
        T: Default + DeserializeOwned,
    {
        let mut all_items = Vec::new();
        let mut page = 1;
        let per_page = 1000;

        loop {
            let page_resp = self
                .list()
                .page(page)
                .per_page(per_page)
                .call::<T>()
                .await?;

            all_items.extend(page_resp.items.into_iter());

            if all_items.len() == page_resp.total_items as usize {
                break;
            }

            page += 1;
        }

        Ok(all_items)
    }
}
