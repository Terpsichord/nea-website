CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    github_id INT NOT NULL UNIQUE,
    username VARCHAR(40) NOT NULL UNIQUE,
    picture_url VARCHAR(255) NOT NULL,
    bio VARCHAR(100) NOT NULL DEFAULT '',
    join_date DATE NOT NULL DEFAULT CURRENT_DATE
);

CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    lang VARCHAR(10) NOT NULL,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    repo_name VARCHAR(255) NOT NULL,
    readme TEXT NOT NULL DEFAULT '',
    public BOOLEAN NOT NULL,
    upload_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_modified TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, repo_name)
);

CREATE TABLE project_tags (
    project_id INT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag VARCHAR(30) NOT NULL,
    PRIMARY KEY (project_id, tag)
);

CREATE TABLE follows (
    follower_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    followee_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (follower_id, followee_id),
    CHECK (follower_id != followee_id)
);

CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    contents VARCHAR(100) NOT NULL,
    user_id INT REFERENCES users(id) ON DELETE CASCADE,
    project_id INT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_id INT REFERENCES comments(id) ON DELETE CASCADE,
    upload_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE likes (
    user_id INT NOT NULL,
    project_id INT NOT NULL,
    PRIMARY KEY (user_id, project_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE VIEW project_info
AS SELECT 
    p.id,
    u.username,
    u.github_id,
    u.picture_url,
    ('https://github.com/' || u.username || '/' || p.repo_name) as github_url,
    ARRAY_REMOVE(ARRAY_AGG(t.tag), NULL) as tags,
    (SELECT COUNT(*) FROM likes WHERE likes.project_id = p.id) as like_count
FROM projects p
LEFT JOIN project_tags t ON t.project_id = p.id
INNER JOIN users u ON p.user_id = u.id
GROUP BY p.id, u.username, u.github_id, u.picture_url;

CREATE TABLE color_schemes (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
);

CREATE TABLE editor_settings (
    user_id INT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    color_scheme INT REFERENCES color_schemes(id),
    auto_save BOOLEAN,
    format_on_save BOOLEAN
);

CREATE TABLE interactions (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id INT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    type VARCHAR(16),
    time TIMESTAMPTZ
);

CREATE TABLE rec_categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64)
);

CREATE TABLE recs (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id INT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    category_id INT NOT NULL REFERENCES rec_categories(id) ON DELETE CASCADE,
    score REAL
);