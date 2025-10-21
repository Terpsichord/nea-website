use std::ops::Deref;

use sqlx::PgPool;

use crate::github::GithubUser;

#[derive(Clone)]
pub struct DatabaseConnector(PgPool);

impl Deref for DatabaseConnector {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
}