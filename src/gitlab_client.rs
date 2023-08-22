use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use reqwest::{Client, header};
use serde::Deserialize;
use tokio::sync::mpsc;

pub struct GitlabClient {
    client: Client,
    url: String,
    token: String,

    pub verbose: bool,
}

impl GitlabClient {
    pub fn new_unsecure(url: &str, token: &str, timeout_seconds: u64) -> GitlabClient {
        if url.is_empty() {
            panic!("url cannot be empty")
        }

        if token.is_empty() {
            panic!("token cannot be empty")
        }

        GitlabClient {
            url: url.to_string(),
            token: token.to_string(),
            verbose: false,
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .danger_accept_invalid_certs(true)
                .build().expect("fail to build client"),
        }
    }

    pub async fn search_by_ids(&self, instance: &Arc<GitlabClient>, ids: Vec<u64>, keywords: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {

        let projects = self.get_project_by_ids(&instance, ids).await?;

        let res = instance.search(&instance, projects, keywords).await?;

        Ok(res)
    }

    pub async fn search_by_group_ids(&self, instance: &Arc<GitlabClient>, group_ids: Vec<u64>, keywords: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {

        let projects = self.get_project_by_group_ids(&instance, group_ids).await?;

        let res = instance.search(&instance, projects, keywords).await?;

        Ok(res)
    }

    pub async fn search_by_name(&self, instance: &Arc<GitlabClient>, project_name: &str, keywords: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {

        let projects = self.get_project_by_name(project_name).await?;

        let res = instance.search(&instance, projects, keywords).await?;

        Ok(res)
    }

    async fn get_project_by_group_ids(&self, instance: &Arc<GitlabClient>, group_ids: Vec<u64>) -> Result<Vec<Project>, Box<dyn Error>> {

        let (tx, mut rx) = mpsc::channel(group_ids.len());

        let mut result = Vec::new();

        for id in group_ids {

            let s = instance.clone();
            let sender = tx.clone();

            tokio::spawn(async move {

                let url = format!("{}/api/v4/groups/{}/projects", s.url, id);

                let res = s.continue_fetch(&url, false).await;

                sender.send(res).await
            });
        }

        drop(tx);

        while let Some(i) = rx.recv().await {
            result.extend(i)
        }

        Ok(result)
    }

    async fn get_project_by_name(&self, project_name: &str) -> Result<Vec<Project>, Box<dyn Error>> {

        let url = format!("{}/api/v4/search?scope=projects&search={}", self.url, project_name);

        let result = self.continue_fetch(&url, true).await;

        Ok(result)
    }

    async fn continue_fetch(&self, url: &str, is_in_query: bool) -> Vec<Project> {

        let mut result = Vec::new();

        let mut page = 0;
        let rows = 100;

        let token = format!("Bearer {}", self.token);

        loop {

            page += 1;

            let mut join = "?";

            if is_in_query {
                join = "&";
            }

            let url = format!("{}{}per_page={}&page={}", url, join, rows, page);
            // println!("{}", url);

            let mut headers = header::HeaderMap::new();
            headers.insert(header::AUTHORIZATION, token.parse().unwrap());

            let res = self.client.get(url).headers(headers).send().await.unwrap().json().await;

            match res {
                Ok(vs) => {

                    let projects: Vec<Project> = vs;

                    result.extend(projects.clone());

                    if projects.len() <= rows {
                        break;
                    }
                }
                Err(err) => {
                    println!("{}", err)
                }
            }
        }

        result
    }

    async fn get_project_by_ids(&self, instance: &Arc<GitlabClient>, ids: Vec<u64>) -> Result<Vec<Project>, Box<dyn Error>> {

        let (tx, mut rx) = mpsc::channel(ids.len());

        for id in ids {
            let s = instance.clone();
            let sender = tx.clone();

            tokio::spawn(async move {

                let token = format!("Bearer {}", s.token);

                let url = format!("{}/api/v4/projects/{}", s.url, id);
                // println!("{}", url);

                let mut headers = header::HeaderMap::new();
                headers.insert(header::AUTHORIZATION, token.parse().unwrap());

                let res = s.client.get(url).headers(headers).send().await.unwrap().json().await;

                match res {
                    Ok(v) => {
                        let project: Project = v;
                        sender.send(project).await
                    }
                    Err(err) => {
                        panic!("{}", err);
                    }
                }.unwrap();

            });
        }

        drop(tx);

        let mut result: Vec<Project> = Vec::new();

        while let Some(i) = rx.recv().await {
            result.push(i)
        }

        Ok(result)
    }



    pub async fn search(&self, instance: &Arc<GitlabClient>, projects: Vec<Project>, keywords: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        let (tx, mut rx) = mpsc::channel(projects.len());

        let wd = get_len(&projects);

        if self.verbose {

            println!("Searching in {} project(s) ...", projects.len());

            println!("{:<15}{:<width$}{:<15}{:<10}{:<20}", "id", "project", "took (ms)", "result", "error", width = wd);
            println!("{:<15}{:<width$}{:<15}{:<10}{:<20}", "--", "-------", "---------", "----", "-----", width = wd);
        }

        for p in projects.clone() {
            let s = instance.clone();
            let sender = tx.clone();
            let query = keywords.to_string();
            let verbose = self.verbose;

            tokio::spawn(async move {

                let start = SystemTime::now();

                let mut sr = SearchResult {
                    id: p.id,
                    name: p.name,
                    count: 0,
                    error: String::from(""),
                    debug_url: String::from(""),
                    search_blob_list: Vec::new(),
                    result_list: Vec::new(),
                };

                let res = s.search_project(p.id, query, &mut sr).await;

                match res {
                    Ok(res) => {

                        if sr.count!= -1 {
                            sr.count = res.len() as i32;
                        }

                        sr.search_blob_list = res;
                    }
                    Err(err) => {
                        println!("{}", err);
                    }
                };

                let end = SystemTime::now().duration_since(start).unwrap();

                // println!("{:?}, time use = {:?}", sr, end);

                if verbose {
                    println!("{:<15}{:<width$}{:<15}{:<10}{:<20}", sr.id, sr.name, end.as_millis(), sr.search_blob_list.len(), sr.error, width = wd);
                }


                sender.send(sr).await
            });
        }

        drop(tx);

        let mut result: Vec<SearchResult> = Vec::new();

        while let Some(sr) = rx.recv().await {
            result.push(sr.clone());
        }

        for p in &projects {
            for sr in &mut result {

                if sr.id == p.id {

                    let mut list = Vec::new();

                    for sb in &sr.search_blob_list {

                        let url = format!("{}/-/blob/{}/{}", p.web_url, sb.ref_, sb.filename);
                        let name = p.name.clone();
                        let data = sb.data.clone();

                        let rr = ReturnResult { name, url, data };

                        list.push(rr);
                    }

                    sr.result_list = list;
                }
            }
        }

        Ok(result)
    }

    async fn search_project(&self, id: i64, keywords: String, sr: &mut SearchResult) -> Result<Vec<SearchBlob>, Box<dyn Error + Send + Sync>>  {

        let mut result = Vec::new();

        let mut page = 0;
        let rows = 100;

        let token = format!("Bearer {}", self.token);

        loop {

            page += 1;

            let url = format!("{}/api/v4/projects/{}/search?scope=blobs&search={}&per_page={}&page={}", self.url, id, keywords, rows, page);
            // println!("{}", url);

            let mut headers = header::HeaderMap::new();
            headers.insert(header::AUTHORIZATION, token.parse().unwrap());

            let res = self.client.get(url).headers(headers).send().await;

            match res {
                Ok(res) => {
                    let sb_list: Vec<SearchBlob> = res.json().await?;
                    result.extend(sb_list.clone());

                    if sb_list.len() <= rows {
                        break;
                    }
                },
                Err(err) => {

                    sr.count = -1;
                    sr.error = err.to_string();

                    // let url = format!("{}/api/v4/projects/{}/search?scope=blobs&search={}", self.url, id, keywords);
                    // sr.debug_url = url;
                }
            }
        }

        Ok(result)
    }

}

fn get_len(projects: &Vec<Project>) -> usize {

    let mut max = 30;

    for p in projects {

        if p.name.len() > max {
            max = p.name.len();
        }
    }

    return max;
}

#[derive(Deserialize, Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub web_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchBlob {
    pub project_id: i64,
    pub data: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: i64,
    pub name: String,
    pub count: i32,
    pub error: String,
    pub debug_url: String,
    pub search_blob_list: Vec<SearchBlob>,
    pub result_list: Vec<ReturnResult>,
}

#[derive(Debug, Clone)]
pub struct ReturnResult {
    pub name: String,
    pub url: String,
    pub data: String,
}






