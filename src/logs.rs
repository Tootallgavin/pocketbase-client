use crate::client::{Client};
use crate::httpc::Httpc;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

pub struct LogsManager<'a, A> {
    pub client: &'a Client<A>,
}

#[derive(Debug, Clone)]
pub struct LogListRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub page: i32,
    pub per_page: i32,
    pub sort: Option<&'a str>,
    pub filter: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct LogViewRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub id: &'a str,
}

#[derive(Debug, Clone)]
pub struct LogStatisticsRequestBuilder<'a, A> {
    pub client: &'a Client<A>,
    pub filter: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogListItem {
    pub id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub url: String,
    pub method: String,
    pub status: i32,
    pub ip: Option<String>,
    pub referer: String,
    pub user_agent: String,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogList {
    pub page: i32,
    pub per_page: i32,
    pub total_items: i32,
    pub items: Vec<LogListItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogStatDataPoint {
    pub total: i32,
    pub date: String,
}

impl<'a, A: Clone> LogStatisticsRequestBuilder<'a, A> {
    pub fn filter(&self, filter_query: &'a str) -> Self {
        Self {
            filter: Some(filter_query),
            ..self.clone()
        }
    }

    pub async fn call(&self) -> Result<Vec<LogStatDataPoint>> {
        let url = format!("{}/api/logs/requests/stats", self.client.base_url);
        let mut build_opts = Vec::new();
        if let Some(filter_opts) = &self.filter {
            build_opts.push(("filter", filter_opts.to_owned()));
        }

        match Httpc::get(self.client, &url, Some(build_opts)).await {
            Ok(result) => {
                let response = result.json::<Vec<LogStatDataPoint>>().await?;
                Ok(response)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'a, A> LogViewRequestBuilder<'a, A> {
    pub async fn call(&self) -> Result<LogListItem> {
        let url = format!("{}/api/logs/requests/{}", self.client.base_url, self.id);
        match Httpc::get(self.client, &url, None).await {
            Ok(result) => {
                let response = result.json::<LogListItem>().await?;
                Ok(response)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'a, A: Clone> LogListRequestBuilder<'a, A> {
    pub fn page(&self, page_count: i32) -> Self {
        LogListRequestBuilder {
            page: page_count,
            ..self.clone()
        }
    }

    pub fn per_page(&self, per_page_count: i32) -> Self {
        LogListRequestBuilder {
            per_page: per_page_count,
            ..self.clone()
        }
    }

    pub fn filter(&self, filter_opts: &'a str) -> Self {
        LogListRequestBuilder {
            filter: Some(filter_opts),
            ..self.clone()
        }
    }

    pub fn sort(&self, sort_opts: &'a str) -> Self {
        LogListRequestBuilder {
            sort: Some(sort_opts),
            ..self.clone()
        }
    }

    pub async fn call(&self) -> Result<LogList> {
        let url = format!("{}/api/logs/requests", self.client.base_url);
        let mut build_opts = Vec::new();

        if let Some(sort_opts) = &self.sort {
            build_opts.push(("sort", sort_opts.to_owned()))
        }
        if let Some(filter_opts) = &self.filter {
            build_opts.push(("filter", filter_opts.to_owned()))
        }
        let per_page_opts = self.per_page.to_string();
        let page_opts = self.page.to_string();
        build_opts.push(("perPage", per_page_opts.as_str()));
        build_opts.push(("page", page_opts.as_str()));

        match Httpc::get(self.client, &url, Some(build_opts)).await {
            Ok(result) => {
                let response = result.json::<LogList>().await?;
                Ok(response)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'a, A: Clone> LogsManager<'a, A> {
    pub fn list(&self) -> LogListRequestBuilder<'a, A> {
        LogListRequestBuilder {
            client: self.client,
            page: 1,
            per_page: 100,
            sort: None,
            filter: None,
        }
    }

    pub fn view(&self, id: &'a str) -> LogViewRequestBuilder<'a, A> {
        LogViewRequestBuilder {
            client: self.client,
            id,
        }
    }

    pub fn statistics(&self) -> LogStatisticsRequestBuilder<'a, A> {
        LogStatisticsRequestBuilder {
            client: self.client,
            filter: None,
        }
    }
}
