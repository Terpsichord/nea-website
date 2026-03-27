use std::iter;

use anyhow::anyhow;
use axum::http::HeaderValue;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use bytes::Bytes;
use chrono::Local;
use reqwest::{RequestBuilder, Response, StatusCode, header::USER_AGENT};
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::{AppError, GithubUserError},
    github::access_tokens::{TokenRequestType, WithTokens, update_tokens},
    lang::ProjectLang,
};

pub mod access_tokens;

// Class that provides a wrapper pattern around GitHub API requests
#[derive(Clone, Debug)]
pub struct GithubClient {
    // HTTP client is stored as a field for making requests
    client: reqwest::Client,
}


impl Default for GithubClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .default_headers(
                    iter::once((USER_AGENT, HeaderValue::from_static(Self::USER_AGENT))).collect(),
                )
                .build()
                .unwrap(),
        }
    }
}

impl GithubClient {
    const API_BASE: &str = "https://api.github.com";
    const USER_AGENT: &str = "nea-website";

    // Convert an API path into a GitHub URL
    fn api_url(path: &str) -> String {
        format!("{}{}", Self::API_BASE, path)
    }

    // Sends a request to the GitHub API using the given access token for authentication
    // If authentication fails, the refresh token is used to retrieve a new access token and retry the request
    async fn send_authenticated(
        &self,
        mut req: RequestBuilder,
        access_token: &str,
        refresh_token: Option<&str>,
    ) -> Result<WithTokens<Response>, AppError> {
        let req_clone = req.try_clone();

        req = req.header("Authorization", format!("Bearer {access_token}"));

        let mut resp = req.send().await.map_err(AppError::other)?;

        let mut new_tokens = None;

        // If new access token needed..
        if let Some(refresh) = refresh_token
            && resp.status() == StatusCode::UNAUTHORIZED
        {
            // get new tokens
            let tokens = self
                .get_tokens(TokenRequestType::Refresh {
                    refresh_token: refresh.to_string(),
                    grant_type: (),
                })
                .await?;

            // retry request with new credentials
            if let Some(req) = req_clone {
                resp = req
                    .header(
                        "Authorization",
                        format!("Bearer {}", tokens.access_unencrypted),
                    )
                    .send()
                    .await
                    .map_err(AppError::other)?;
            }
            new_tokens = Some(tokens);
        }

        // display an error if request failed
        if !resp.status().is_success() {
            println!("{resp:#?}");
        }

        // return new tokens if any
        Ok(WithTokens(resp, new_tokens))
    }

    // Fetches information about the Github user using the access token, and caches the user's id with the encrypted token
    // Returns the user info on a successful fetch
    pub async fn get_user(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
    ) -> Result<WithTokens<GithubUser>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(Self::api_url("/user")),
                access_token,
                refresh_token,
            )
            .await?;

        let user_res = resp
            .json::<GithubUserResponse>()
            .await
            .map_err(AppError::auth_failed)?;

        match user_res {
            GithubUserResponse::User(user) => Ok(WithTokens(user, tokens)),
            GithubUserResponse::Error(error) => Err(AppError::GithubAuth(error)),
        }
    }

    // Fetches the files and directories of a given project
    // returned as a TAR archive compressed using Gzip
    pub async fn get_project_tarball(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<(String, Bytes)>, AppError> {
        // fetch the tarball
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(Self::api_url(&format!(
                    "/repos/{username}/{repo_name}/tarball"
                ))),
                access_token,
                Some(refresh_token),
            )
            .await?;

        // get the name of the tarball
        let disposition_header = resp.headers().get("content-disposition").expect("missing content-disposition").to_str().unwrap();
        let tarball_name = disposition_header
            .split(";")
            .find_map(|s| {
                let s = s.trim();
                if s.starts_with("filename=") {
                    Some(s.trim_start_matches("filename=").trim_matches('"').split('.').next().unwrap())
                } else {
                    None
                }
            })
            .unwrap();

        Ok(WithTokens(
            (tarball_name.to_string(), resp.bytes().await.map_err(AppError::other)?),
            tokens,
        ))
    }

    // Converts a project title is a valid GitHub repo name
    fn sanitize_repo_name(name: &str) -> String {
        // function to check whether a character is allowed in a github repo name
        let is_valid = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';

        // remove all invalid characters and make lowercase
        name.replace(' ', "_")
            .replace(|c| !is_valid(c), "")
            .to_lowercase()
    }

    // Creates a new GitHub repository for a given user
    pub async fn create_repo(
        &self,
        mut access_token: &str,
        mut refresh_token: &str,
        username: &str,
        title: &str,
        lang: ProjectLang,
        private: bool,
    ) -> Result<WithTokens<CreateRepoResponse>, AppError> {
        // create the repo name from the project title
        let repo_name = Self::sanitize_repo_name(title);

        // check whether a repo with the repo name already exists for the passed user
        let WithTokens(exists, mut tokens) = self
            .repo_exists(access_token, refresh_token, username, &repo_name)
            .await?;

        // if it does exist, return
        if exists {
            return Ok(WithTokens(
                CreateRepoResponse {
                    repo_name,
                    already_exists: true,
                },
                tokens,
            ));
        }
        update_tokens!(access_token, refresh_token, tokens);

        // send a POST request to create a new repo
        let WithTokens(resp, new_tokens) = self
            .send_authenticated(
                self.client
                    .post(Self::api_url("/user/repos"))
                    .json(&json!({ "name": repo_name, "private": private })),
                access_token,
                Some(refresh_token),
            )
            .await?;

        if let Some(new_tokens) = new_tokens {
            tokens = Some(new_tokens);
            (access_token, refresh_token) = tokens.as_ref().unwrap().unencrypted();
        }

        // display error messages
        if !resp.status().is_success() {
            return Err(AppError::other(anyhow!(
                "failed to create project: {}",
                resp.text().await.unwrap()
            )));
        }

        // add required initial files to the new repo 
        let WithTokens((), new_tokens) = self
            .add_repo_files(access_token, refresh_token, username, &repo_name, lang)
            .await?;
        tokens = new_tokens.or(tokens);

        Ok(WithTokens(
            CreateRepoResponse {
                repo_name,
                already_exists: false,
            },
            tokens,
        ))
    }

    // Sends a request to GitHub to check whether a repo with the name exist
    pub async fn repo_exists(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<bool>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client
                    .get(Self::api_url(&format!("/repos/{username}/{repo_name}"))),
                access_token,
                Some(refresh_token),
            )
            .await?;

        let exists = resp.status().is_success();

        Ok(WithTokens(exists, tokens))
    }

    // Add initial required files to GitHub repo
    async fn add_repo_files(
        &self,
        mut access_token: &str,
        mut refresh_token: &str,
        username: &str,
        repo_name: &str,
        lang: ProjectLang,
    ) -> Result<WithTokens<()>, AppError> {
        // get the project.toml that will be added to the repo for the given language
        let project_toml = lang.get_project_toml().map_err(AppError::other)?;
        let WithTokens((), tokens) = self
            .add_file(
                access_token,
                refresh_token,
                username,
                repo_name,
                ".ide/project.toml",
                &project_toml,
            )
            .await?;
        update_tokens!(access_token, refresh_token, tokens);

        // get starting file for language 
        let (init_path, init_content) = lang.get_initial_file().map_err(AppError::other)?;
        let WithTokens((), new_tokens) = self
            .add_file(
                access_token,
                refresh_token,
                username,
                repo_name,
                init_path,
                &init_content,
            )
            .await?;

        Ok(WithTokens((), new_tokens.or(tokens)))
    }

    // Add an individual file to the GitHub repo
    async fn add_file(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
        path: &str,
        content: &str,
    ) -> Result<WithTokens<()>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client
                    .put(Self::api_url(&format!(
                        "/repos/{username}/{repo_name}/contents/{path}"
                    )))
                    .json(&json!({
                        "message": format!("Add {path}"),
                        "content": BASE64_STANDARD.encode(content)
                    })),
                access_token,
                Some(refresh_token),
            )
            .await?;

        if resp.status().is_success() {
            Ok(WithTokens((), tokens))
        } else {
            Err(AppError::other(anyhow!(
                "failed to add file: {}",
                resp.text().await.unwrap()
            )))
        }
    }

    // Add multiple files to the GitHub repo with a single `git` commit
    // Requires interfacing with the Git Database API, as described in this article: 
    pub async fn add_multiple_files(&self, mut access_token: &str, mut refresh_token: &str, username: &str, repo_name: &str, files: Vec<(String, String)>) -> Result<WithTokens<()>, AppError> {
        // create binary objects (blobs) that represent each file that will be added in the commit
        println!("creating blobs");
        let mut blob_shas = vec![];
        for (_, content) in &files {
            let WithTokens(resp, _) = self.send_authenticated(
                self.client
                    .post(Self::api_url(&format!("/repos/{username}/{repo_name}/git/blobs")))
                    .json(&json!({
                        "content": content,
                        "encoding": "utf-8"
                    })),
                access_token,
                Some(refresh_token),
            )
            .await?;

            let sha_hash = resp
                .json::<GithubShaResponse>()
                .await
                .map_err(AppError::other)?
                .sha;

            // add the SHA hash for each file into list so that they can be add to the Git tree later
            blob_shas.push(sha_hash);
        }

        // create a git tree which represents the file tree structure of the entire contents of the new commit
        println!("creating tree");
        let json = json!({
                    "tree": files.iter().zip(blob_shas).map(|((path, _), sha_hash)| json!({
                        "path": path,
                        "mode": "100644",
                        "type": "blob",
                        "sha": sha_hash
                    })).collect::<Vec<_>>(),
                });

        println!("send tree json: {:#?}", &json);

        // add the tree using the git db api
        let WithTokens(resp, tokens) = self.send_authenticated(
            self.client
                .post(Self::api_url(&format!("/repos/{username}/{repo_name}/git/trees")))
                .json(&json),
            access_token,
            Some(refresh_token),
        )
        .await?;
        update_tokens!(access_token, refresh_token, tokens);

        let tree_sha = resp
            .json::<GithubShaResponse>()
            .await
            .map_err(AppError::other)?
            .sha;

        // get the tree for the previous (parent) commit
        println!("getting parent tree");
        let WithTokens(resp, tokens) = self.send_authenticated(
            self.client
                .get(Self::api_url(&format!("/repos/{username}/{repo_name}/git/refs/heads/main"))),
            access_token,
            Some(refresh_token),
        )
        .await?;
        update_tokens!(access_token, refresh_token, tokens);

        let parent_sha = resp
            .json::<GithubBranchResponse>()
            .await
            .map_err(AppError::other)?
            .object
            .sha;

        // add the new commit, with the new tree and parent commit's tree
        println!("creating commit");
        let WithTokens(resp, tokens) = self.send_authenticated(
            self.client
                .post(Self::api_url(&format!("/repos/{username}/{repo_name}/git/commits")))
                .json(&json!({
                    "message": format!("Save from IDE {}", Local::now().to_rfc3339()),
                    "tree": tree_sha,
                    "parents": [parent_sha],
                })),
            access_token,
            Some(refresh_token),
        )
        .await?;
        update_tokens!(access_token, refresh_token, tokens);

        let commit_sha = resp
            .json::<GithubShaResponse>()
            .await
            .map_err(AppError::other)?
            .sha;

        // update the main branch to point to the newly created commit as being the newest commit
        let WithTokens(resp, tokens) = self.send_authenticated(
            self.client
                .patch(Self::api_url(&format!("/repos/{username}/{repo_name}/git/refs/heads/main")))
                .json(&json!({
                    "sha": commit_sha
                })),
            access_token,
            Some(refresh_token),
        )
        .await?;
        let status = resp.status();

        let updated_branch_sha = resp
            .json::<GithubBranchResponse>()
            .await
            .map_err(AppError::other)?
            .object
            .sha;

        // check whether the commit succeeded or not
        if !status.is_success() || updated_branch_sha != commit_sha {
            return Err(AppError::other(anyhow!("failed to update branch")));
        }

        Ok(WithTokens((), tokens))
    }

    // Get the README.md file from a given GitHub repo
    pub async fn get_readme(
        &self,
        mut access_token: &str,
        mut refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<String>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(Self::api_url(&format!(
                    "/repos/{username}/{repo_name}/readme"
                ))),
                access_token,
                Some(refresh_token),
            )
            .await?;

        let readme_resp = resp
            .json::<GithubReadmeResponse>()
            .await
            .map_err(AppError::other)?;
        update_tokens!(access_token, refresh_token, tokens);

        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(readme_resp.download_url),
                access_token,
                Some(refresh_token),
            )
            .await?;

        let readme = resp.text().await.map_err(AppError::other)?;

        Ok(WithTokens(readme, tokens))
    }

    // Create a fork of a GitHub repo (to allow for remixing projects)
    pub async fn fork_repo(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<()>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.post(Self::api_url(&format!(
                    "/repos/{username}/{repo_name}/forks"
                ))),
                access_token,
                Some(refresh_token),
            )
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::other(anyhow!(
                "failed to fork repo: {}",
                resp.text().await.unwrap()
            )));
        }

        Ok(WithTokens((), tokens))
    }

    // Check whether a user has the GitHub App for this app installed to their account or not
    pub async fn user_installed(&self, access_token: &str) -> Result<bool, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(Self::api_url("/user/installations")),
                access_token,
                None,
            )
            .await?;

        // ensure that the App has at least 1 installation on the account
        
        let installation_count = resp
            .json::<InstallationsResponse>()
            .await
            .map_err(AppError::other)?
            .total_count;

        Ok(installation_count > 0)
    }
}

// Structs which are used to data from JSON received in GitHub API responses

#[derive(Deserialize)]
pub struct GithubUser {
    pub id: i32,
    #[serde(rename = "login")]
    pub username: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum GithubUserResponse {
    User(GithubUser),
    Error(GithubUserError),
}

#[derive(Deserialize)]
struct GithubReadmeResponse {
    download_url: String,
}

pub struct CreateRepoResponse {
    pub repo_name: String,
    pub already_exists: bool,
}

#[derive(Deserialize)]
struct InstallationsResponse {
    total_count: u32,
}

#[derive(Deserialize)]
struct GithubShaResponse {
    sha: String,
}

#[derive(Deserialize)]
struct GithubBranchResponse {
    object: GithubShaResponse,
}
