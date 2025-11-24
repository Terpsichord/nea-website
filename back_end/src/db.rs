use std::ops::Deref;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{api::ProjectInfo, error::AppError, github::GithubUser};

#[derive(Clone)]
pub struct DatabaseConnector(PgPool);

impl Deref for DatabaseConnector {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Project {
    pub id: i32,
    pub user_id: i32,
    pub info: ProjectInfo,
    pub github_url: String,
    pub upload_time: DateTime<Utc>,
    pub public: bool,
    pub owned: bool,
}

impl DatabaseConnector {
    pub const fn new(pool: PgPool) -> Self {
        Self(pool)
    }

    pub async fn add_user(&self, user: &GithubUser) -> sqlx::Result<()> {
        let existing_user = sqlx::query!("SELECT * FROM users WHERE github_id = $1", user.id)
            .fetch_optional(&self.0)
            .await?;

        if existing_user.is_none() {
            sqlx::query!(
                "INSERT INTO users (github_id, username, picture_url) VALUES ($1, $2, $3)",
                user.id,
                user.username,
                user.avatar_url,
            )
            .execute(&self.0)
            .await?;
        }

        Ok(())
    }

    pub async fn get_project(
        &self,
        username: &str,
        repo_name: &str,
        github_id: Option<i32>,
        must_own: bool,
    ) -> Result<Project, AppError> {
        let project = sqlx::query_as!(
            Project,
            r#"
            SELECT 
                p.id,
                p.user_id,
                (p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo",
                pi.github_url as "github_url!",
                p.upload_time,
                p.public,
                pi.github_id = $3 as "owned!"
            FROM projects p
            INNER JOIN project_info pi ON pi.id = p.id
            WHERE pi.username = $1
            AND p.repo_name = $2
            AND (pi.github_id = $3 OR (p.public AND NOT $4))
            "#,
            username,
            repo_name,
            github_id,
            must_own,
        ).fetch_one(&self.0).await?;

        // TODO: (i think), make this fetch 0 or 1 and show error if 0

        Ok(project)
    }
}
