use crate::client::{Client};
use crate::httpc::Httpc;
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde::{de::DeserializeOwned, Deserialize};

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
        build_opts.push(("per_page", per_page_opts.as_str()));
        build_opts.push(("page", page_opts.as_str()));

        match Httpc::get(self.client, &url, Some(build_opts)).await {
            Ok(result) => {
                let response = result.json::<RecordList<T>>().await?;
                Ok(response)
            }
            Err(e) => Err(e),
        }
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
    pub async fn call<T: Default + DeserializeOwned>(&self) -> Result<T> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.client.base_url, self.collection_name, self.identifier
        );
        match Httpc::get(self.client, &url, None).await {
            Ok(result) => {
                let response = result.json::<T>().await?;
                Ok(response)
            }
            Err(e) => Err(anyhow!("error: {}", e)),
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
}
