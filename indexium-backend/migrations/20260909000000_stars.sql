CREATE TABLE stars (
    github_id BIGINT,
    mod_id UUID REFERENCES mods(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (github_id, mod_id)
);
