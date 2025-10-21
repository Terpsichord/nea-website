
fn fetch_project_files(client: &reqwest::Client, access_token: &str, username: &str, repo_name: &str) -> Result<PathBuf, AppError> {
    let res = client
        .get("https://api.github.com/")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(AppError::auth_failed)?
        .json::<GithubUserResponse>()
        .await
        .map_err(AppError::auth_failed)?;
}