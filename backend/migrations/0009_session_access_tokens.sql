-- Access credentials belong to the same stable session/user ownership boundary
-- as refresh credentials, but are not part of the refresh rotation graph.
CREATE TABLE session_access_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ(6) NOT NULL,
    CONSTRAINT session_access_tokens_hash_unique UNIQUE (token_hash),
    CONSTRAINT session_access_tokens_session_owner FOREIGN KEY (session_id, user_id)
        REFERENCES user_sessions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT session_access_tokens_hash_length CHECK (octet_length(token_hash) = 32),
    CONSTRAINT session_access_tokens_expiry CHECK (expires_at > created_at),
    CONSTRAINT session_access_tokens_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT session_access_tokens_expires_at_range CHECK (
        expires_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND expires_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);

CREATE INDEX session_access_tokens_session_id_idx ON session_access_tokens (session_id);
CREATE INDEX session_access_tokens_user_id_idx ON session_access_tokens (user_id);
