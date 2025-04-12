CREATE TABLE IF NOT EXISTS users (
    id INT NOT NULL AUTO_INCREMENT,
    github_id INT NOT NULL,
    username VARCHAR(40) NOT NULL,
    picture_url VARCHAR(255) NOT NULL,
    bio VARCHAR(100) NOT NULL DEFAULT '',
    join_date DATE NOT NULL DEFAULT (CURRENT_DATE),
    PRIMARY KEY (id),
    UNIQUE (username)
);

CREATE TABLE IF NOT EXISTS projects (
    id INT NOT NULL AUTO_INCREMENT,
    title VARCHAR(255) NOT NULL,
    user_id INT NOT NULL,
    github_url VARCHAR(255) NOT NULL,
    readme TEXT,
    public BOOLEAN,
    upload_time TIMESTAMP,
    last_modified TIMESTAMP,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS project_tags (
    project_id INT NOT NULL,
    tag VARCHAR(30),
    PRIMARY KEY (project_id, tag),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS follows (
    follower_id INT NOT NULL,
    followee_id INT NOT NULL,
    PRIMARY KEY (follower_id, followee_id),
    FOREIGN KEY (follower_id) REFERENCES users(id),
    FOREIGN KEY (followee_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS comments (
    id INT NOT NULL AUTO_INCREMENT,
    contents VARCHAR(100),
    user_id INT,
    project_id INT NOT NULL,
    parent_id INT,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (parent_id) REFERENCES comments(id)
);

CREATE TABLE IF NOT EXISTS likes (
    user_id INT NOT NULL,
    project_id INT NOT NULL,
    PRIMARY KEY (user_id, project_id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS color_schemes (
    id INT NOT NULL AUTO_INCREMENT,
    -- TODO: add more columns here
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS editor_configs (
    user_id INT NOT NULL,
    color_schemes INT,
    -- TODO: add more columns here
    FOREIGN KEY (user_id) REFERENCES users(id)
);
