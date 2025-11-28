use std::iter;

use anyhow::anyhow;
use axum::http::HeaderValue;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use bytes::Bytes;
use reqwest::{RequestBuilder, Response, StatusCode, header::USER_AGENT};
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::{AppError, GithubUserError},
    github::access_tokens::{TokenRequestType, WithTokens},
    lang::ProjectLang,
};

pub mod access_tokens;

#[derive(Clone, Debug)]
pub struct GithubClient {
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

impl GithubClient {
    const API_BASE: &str = "https://api.github.com";
    const USER_AGENT: &str = "nea-website";

    fn api_url(path: &str) -> String {
        format!("{}{}", Self::API_BASE, path)
    }

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
        if let Some(refresh) = refresh_token
            && resp.status() == StatusCode::UNAUTHORIZED
        {
            let tokens = self
                .get_tokens(TokenRequestType::Refresh {
                    refresh_token: refresh.to_string(),
                    grant_type: (),
                })
                .await?;
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

        Ok(WithTokens(resp, new_tokens))
    }

    /// Fetches information about the Github user using the access token, and caches the user's id with the encrypted token
    ///
    /// Returns the user info on a successful fetch
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

    pub async fn get_project_tarball(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<Bytes>, AppError> {
        let WithTokens(resp, tokens) = self
            .send_authenticated(
                self.client.get(Self::api_url(&format!(
                    "/repos/{username}/{repo_name}/tarball"
                ))),
                access_token,
                Some(refresh_token),
            )
            .await?;

        Ok(WithTokens(
            resp.bytes().await.map_err(AppError::other)?,
            tokens,
        ))
    }

    fn sanitize_repo_name(name: &str) -> String {
        // function to check whether character is allowed in a github repo name
        let is_valid = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';

        // remove all invalid characters and make lowercase
        name.replace(' ', "_")
            .replace(|c| !is_valid(c), "")
            .to_lowercase()
    }

    pub async fn create_repo(
        &self,
        mut access_token: &str,
        mut refresh_token: &str,
        username: &str,
        title: &str,
        lang: ProjectLang,
        private: bool,
    ) -> Result<WithTokens<CreateRepoResponse>, AppError> {
        let repo_name = Self::sanitize_repo_name(title);

        let WithTokens(exists, mut tokens) = self
            .repo_exists(access_token, refresh_token, username, &repo_name)
            .await?;

        if exists {
            return Ok(WithTokens(
                CreateRepoResponse {
                    repo_name,
                    already_exists: true,
                },
                tokens,
            ));
        }

        if let Some(ref tokens) = tokens {
            (access_token, refresh_token) = tokens.unencrypted();
        }

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
            // FIXME: it's so stupid that this works
            (access_token, refresh_token) = tokens.as_ref().unwrap().unencrypted();
        }

        if !resp.status().is_success() {
            return Err(AppError::other(anyhow!(
                "failed to create project: {}",
                resp.text().await.unwrap()
            )));
        }

        let WithTokens((), new_tokens) = self.add_repo_files(access_token, refresh_token, username, &repo_name, lang).await?;
        tokens = new_tokens.or(tokens);

        Ok(WithTokens(
            CreateRepoResponse {
                repo_name,
                already_exists: false,
            },
            tokens,
        ))
    }

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

    async fn add_repo_files(
        &self,
        mut access_token: &str,
        mut refresh_token: &str,
        username: &str,
        repo_name: &str,
        lang: ProjectLang,
    ) -> Result<WithTokens<()>, AppError> {
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
        if let Some(ref tokens) = tokens {
            (access_token, refresh_token) = tokens.unencrypted();
        }

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

        if let Some(ref tokens) = tokens {
            (access_token, refresh_token) = tokens.unencrypted();
        }

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
    
    pub async fn fork_repo(
        &self,
        access_token: &str,
        refresh_token: &str,
        username: &str,
        repo_name: &str,
    ) -> Result<WithTokens<()>, AppError> {
        let WithTokens(resp, tokens) = self.send_authenticated(
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
}

#[derive(Deserialize)]
struct GithubReadmeResponse {
    download_url: String,
}

pub struct CreateRepoResponse {
    pub repo_name: String,
    pub already_exists: bool,
}
