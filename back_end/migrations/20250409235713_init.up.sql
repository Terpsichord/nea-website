CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    github_id INT NOT NULL,
    username VARCHAR(40) NOT NULL UNIQUE,
    picture_url VARCHAR(255) NOT NULL,
    bio VARCHAR(100) NOT NULL DEFAULT '',
    join_date DATE NOT NULL DEFAULT CURRENT_DATE
);

CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    user_id INT NOT NULL REFERENCES users(id),
    repo_name VARCHAR(255) NOT NULL,
    readme TEXT NOT NULL DEFAULT '',
    public BOOLEAN NOT NULL,
    upload_time TIMESTAMP NOT NULL,
    last_modified TIMESTAMP,
    UNIQUE (user_id, repo_name)
);

CREATE TABLE project_tags (
    project_id INT NOT NULL REFERENCES projects(id),
    tag VARCHAR(30) NOT NULL,
    PRIMARY KEY (project_id, tag)
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

CREATE TABLE follows (
    follower_id INT NOT NULL REFERENCES users(id),
    followee_id INT NOT NULL REFERENCES users(id),
    PRIMARY KEY (follower_id, followee_id)
    CHECK (follower_id != followee_id)
);

CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    contents VARCHAR(100),
    user_id INT REFERENCES users(id),
    project_id INT NOT NULL REFERENCES projects(id),
    parent_id INT REFERENCES comments(id)
);

CREATE TABLE likes (
    user_id INT NOT NULL,
    project_id INT NOT NULL,
    PRIMARY KEY (user_id, project_id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE color_schemes (
    id SERIAL PRIMARY KEY
    -- TODO: add more columns here
);

CREATE TABLE editor_configs (
    user_id INT NOT NULL REFERENCES users(id),
    color_schemes INT REFERENCES color_schemes(id)
    -- TODO: add more columns here
);