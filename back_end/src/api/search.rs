use std::cmp::max;

use axum::{
    Json, extract::State,
};
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::{debug, instrument};

use crate::{
    api::ProjectResponse,
    db::DatabaseConnector,
    error::AppError,
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Relevant,
    Title,
    Likes,
    UploadTime,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    query: String,
    #[serde(default)]
    tags: Vec<String>,
    lang: Option<String>,
    sort: Option<SortOrder>,
    dir: Option<SortDirection>,
}

impl SearchQuery {
    fn sort_clause(&self) -> String {
        let (Some(sort), Some(dir)) = (self.sort, self.dir) else {
            return String::new();
        };

        "ORDER BY ".to_string()
            + match sort {
                SortOrder::Relevant => return String::new(),
                SortOrder::Title => "p.title",
                SortOrder::Likes => "pi.like_count",
                SortOrder::UploadTime => "p.upload_time",
            }
            + match dir {
                SortDirection::Ascending => " ASC",
                SortDirection::Descending => " DESC",
            }
    }
}

#[instrument(skip(db))]
pub async fn search_projects(
    Query(mut search_query): Query<SearchQuery>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    // remove empty tags
    search_query.tags.retain(|t| !t.is_empty());

    let sort_clause = search_query.sort_clause();
    let query = format!(
        r"
        SELECT
            p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count,
            pi.github_url as github_url,
            p.upload_time,
            p.public,
            false as owned
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE p.public
        AND pi.tags::text[] @> $1
        {}
        {}
    ",
        search_query
            .lang
            .map(|lang| format!("AND p.lang = '{lang}'"))
            .unwrap_or_default(),
        sort_clause
    );

    let projects = sqlx::query_as::<_, ProjectResponse>(&query)
        .bind(&search_query.tags)
        .fetch_all(&*db)
        .await?;

    debug!("projects: {:#?}", projects);

    let filtered_projects: Vec<_> = projects
        .into_iter()
        .filter(|p| {
            !kmp_search(
                &p.info.title.to_lowercase(),
                &search_query.query.to_lowercase(),
            )
            .is_empty()
                || !boyer_moore_search(
                    &p.info.readme.to_lowercase(),
                    &search_query.query.to_lowercase(),
                )
                .is_empty()
        })
        .collect();
    debug!("filtered projects: {:#?}", filtered_projects);

    Ok(Json(filtered_projects))
}

// ========= Search Algorithms =========


// knuth-morris-pratt search

fn create_lps(pattern: &[u8]) -> Vec<usize> {
    let mut lps = vec![0; pattern.len()];

    let mut length = 0;
    let mut i = 1;

    while i < pattern.len() {
        // if there is a match
        if pattern[i] == pattern[length] {
            // longest prefix-suffix length is incremented
            length += 1;
            lps[i] = length;
            i += 1;
        } else if length != 0 {
            // if there is no match, use the previous LPS length
            length = lps[length - 1];
        } else {
            i += 1;
        }
    }

    lps
}

fn kmp_search(text: &str, pattern: &str) -> Vec<usize> {
    let text = text.as_bytes();
    let pattern = pattern.as_bytes();

    let lps = create_lps(pattern);
    let mut result = vec![];

    // current index in the text
    let mut i = 0;

    // current index in the pattern
    let mut j = 0;

    while i < text.len() {
        // if there is a match
        if text[i] == pattern[j] {
            // move to the next index
            i += 1;
            j += 1;

            // if the pattern is fully matched
            if j == pattern.len() {
                // append the index of the match
                result.push(i - j);
                // jump j back to the start of the prefix-suffix
                j = lps[j - 1];
            }
        } else if j != 0 {
            // jump j back
            j = lps[j - 1];
        } else {
            // move to the next index in the text
            i += 1;
        }
    }

    result
}


// boyer-moore search

fn bad_character_heuristic(pattern: &[u8]) -> Vec<i32> {
    let mut bad_char = vec![-1; 256];

    // store the last occurrence of each character in the pattern
    for i in 0..pattern.len() {
        bad_char[pattern[i] as usize] = i as i32;
    }

    bad_char
}

fn boyer_moore_search(text: &str, pattern: &str) -> Vec<usize> {
    let text = text.as_bytes();
    let pattern = pattern.as_bytes();

    let m = pattern.len();
    let n = text.len();

    // preprocess the pattern using the bad character heuristic
    let bad_char = bad_character_heuristic(pattern);

    let mut result = vec![];

    // current shift of the pattern with respect to the text
    let mut s = 0;

    while s as isize <= n as isize - m as isize {
        // start comparing from the right end of the pattern
        let mut j = m as isize - 1;

        // keep moving left while characters match
        while j >= 0 && pattern[j as usize] == text[s + j as usize] {
            j -= 1;
        }

        if j < 0 {
            // pattern fully matched
            result.push(s);

            // shift the pattern to align the next character in the text
            // if possible, use the bad character table for the next character
            if s + m < n {
                s += (m as i32 - bad_char[text[s + m] as usize]) as usize;
            } else {
                s += 1; // move by 1 if past the end of the text
            }
        } else {
            // if mismatch occurred, calculate shift using the bad character rule
            let shift = max(1, j as i32 - bad_char[text[s + j as usize] as usize]) as usize;
            // shift the pattern
            s += shift;
        }
    }

    result
}
