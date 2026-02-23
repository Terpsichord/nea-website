use std::ops::Deref;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ws_messages::EditorSettings;

use crate::{api::ProjectInfo, error::AppError, github::GithubUser, lang::ProjectLang};

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
    pub lang: ProjectLang,
    pub github_url: String,
    pub upload_time: DateTime<Utc>,
    pub public: bool,
    pub owned: bool,
}

pub struct NewProject {
    pub title: String,
    pub repo_name: String,
    pub lang: ProjectLang,
    pub user_id: i32,
    pub readme: String,
    pub public: bool,
    pub tags: Vec<String>,
}

// This class provides a thin wrapper around the database connection pool `PgPool` provided by `sqlx`.
// It provides several convenience functions for interacting with the database.
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

            let settings = EditorSettings::default();

            sqlx::query!(
                r#"
                INSERT INTO editor_settings (user_id, auto_save, format_on_save)
                SELECT id, $1, $2
                FROM users
                WHERE github_id = $3
                "#,
                settings.auto_save,
                settings.format_on_save,
                user.id,
            )
            .execute(&self.0)
            .await?;
        }

        Ok(())
    }

    pub async fn add_project(&self, project: &NewProject) -> sqlx::Result<()> {
        let id: i32 = sqlx::query_scalar!(
            r#"
            INSERT INTO projects (title, lang, user_id, repo_name, readme, public)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            project.title,
            project.lang.to_string(),
            project.user_id,
            project.repo_name,
            project.readme,
            project.public,
        )
        .fetch_one(&self.0)
        .await?;

        if !project.tags.is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO project_tags (project_id, tag)
                VALUES ($1, UNNEST($2::text[]))
                "#,
                id,
                &project.tags
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
                p.lang as "lang: ProjectLang",
                p.upload_time,
                p.public,
                COALESCE(pi.github_id = $3, 'false') as "owned!"
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
        ).fetch_optional(&self.0).await?;

        if let Some(project) = project {
            Ok(project)
        } else {
            Err(AppError::NotFound)
        }
    }

    pub async fn project_exists(&self, user_id: i32, title: &str) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM projects
                WHERE title = $1
                AND user_id = $2
            ) as "exists!"
            "#,
            title,
            user_id
        )
        .fetch_one(&self.0)
        .await?;

        Ok(exists)
    }

    pub async fn get_editor_settings(&self, user_id: i32) -> Result<EditorSettings, AppError> {
        let settings = sqlx::query_as!(
            EditorSettings,
            r#"
            SELECT s.auto_save, c.name as color_scheme, s.format_on_save 
            FROM editor_settings s
            INNER JOIN color_schemes c ON s.color_scheme = c.id
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.0)
        .await?;

        Ok(settings)
    }

    pub async fn update_editor_settings(
        &self,
        user_id: i32,
        settings: EditorSettings,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE editor_settings
            SET auto_save = $1, format_on_save = $2, color_scheme = (
                SELECT id
                FROM color_schemes
                WHERE name = $3
            )
            WHERE user_id = $4
            "#,
            settings.auto_save,
            settings.format_on_save,
            settings.color_scheme,
            user_id
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
}
