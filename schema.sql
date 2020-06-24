
/* DROP TYPE IF EXISTS ROLE; */

/* CREATE TYPE ROLE AS ENUM ('admin', 'tagger'); */
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username    TEXT UNIQUE NOT NULL,
    nickname    TEXT NOT NULL,
    password    TEXT NOT NULL,
    role    TEXT NOT NULL,
    date_created DATE NOT NULL DEFAULT CURRENT_DATE,
    last_modified DATE NOT NULL DEFAULT CURRENT_DATE
);

ALTER TABLE users DROP CONSTRAINT IF EXISTS namechk;
ALTER TABLE users
  ADD CONSTRAINT namechk CHECK (char_length(username) <= 64 AND char_length(username) >= 4);


ALTER TABLE users DROP CONSTRAINT IF EXISTS userchk;
ALTER TABLE users
  ADD CONSTRAINT userchk CHECK (char_length(nickname) <= 64 AND char_length(nickname) >= 4);

CREATE TABLE IF NOT EXISTS albums (
    id SERIAL PRIMARY KEY,
    title VARCHAR(300) NOT NULL,
    description TEXT,
    image_number INT,
    tagged_number INT DEFAULT 0,
    date_created DATE NOT NULL DEFAULT CURRENT_DATE,
    date_modified DATE NOT NULL DEFAULT CURRENT_DATE,
    users_id INT NOT NULL,
    thumbnail TEXT,
    first_photo TEXT,
    FOREIGN KEY (users_id) REFERENCES users (id)
);

CREATE TABLE IF NOT EXISTS is_tagging_album (
    users_id INT NOT NULL,
    albums_id INT NOT NULL,
    in_progress bool NOT NULL,
    PRIMARY KEY ( users_id, albums_id),
    FOREIGN KEY (users_id) REFERENCES users (id),
    FOREIGN KEY (albums_id) REFERENCES albums (id)
);

CREATE TABLE IF NOT EXISTS image_metas (
    id SERIAL PRIMARY KEY,
    albums_id INT NOT NULL,
    date_created DATE NOT NULL DEFAULT CURRENT_DATE,
    date_modified DATE NOT NULL DEFAULT CURRENT_DATE,
    file_path TEXT NOT NULL,
    locked_at TIMESTAMP ,
    coordinates TEXT NOT NULL,
    verified BOOL DEFAULT FALSE,
    tagged BOOL DEFAULT FALSE,
    FOREIGN KEY (albums_id) REFERENCES albums (id)
);

CREATE TABLE IF NOT EXISTS tags (
    id SERIAL PRIMARY KEY,
    name VARCHAR(122) NOT NULL
);

CREATE TABLE IF NOT EXISTS album_tags (
    tags_id INT PRIMARY KEY,
    albums_id INT NOT NULL,
    FOREIGN KEY (albums_id) REFERENCES albums (id),
    FOREIGN KEY (tags_id) REFERENCES tags (id)
);

CREATE TABLE IF NOT EXISTS image_tags (
    tags_id INT PRIMARY KEY,
    image_metas_id INT NOT NULL,
    FOREIGN KEY (tags_id) REFERENCES tags (id),
    FOREIGN KEY (image_metas_id) REFERENCES image_metas (id)
);

