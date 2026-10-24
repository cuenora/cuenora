-- One stable session is also the ownership boundary for a refresh-token family.
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    revoked_at TIMESTAMPTZ(6),
    CONSTRAINT user_sessions_owner UNIQUE (id, user_id),
    CONSTRAINT user_sessions_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT user_sessions_revoked_at_range CHECK (
        revoked_at IS NULL OR (
            revoked_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND revoked_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    )
);

CREATE INDEX user_sessions_user_id_idx ON user_sessions (user_id);

-- Retain consumed generations for the later rotation/reuse protocol.
CREATE TABLE session_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ(6) NOT NULL,
    used_at TIMESTAMPTZ(6),
    replaced_by UUID,
    CONSTRAINT session_refresh_tokens_hash_unique UNIQUE (token_hash),
    CONSTRAINT session_refresh_tokens_owner UNIQUE (id, session_id, user_id),
    CONSTRAINT session_refresh_tokens_session_owner FOREIGN KEY (session_id, user_id)
        REFERENCES user_sessions (id, user_id) ON DELETE RESTRICT,
    CONSTRAINT session_refresh_tokens_successor_owner FOREIGN KEY (replaced_by, session_id, user_id)
        REFERENCES session_refresh_tokens (id, session_id, user_id) ON DELETE RESTRICT,
    CONSTRAINT session_refresh_tokens_hash_length CHECK (octet_length(token_hash) = 32),
    CONSTRAINT session_refresh_tokens_expiry CHECK (expires_at > created_at),
    CONSTRAINT session_refresh_tokens_rotation_pair CHECK ((used_at IS NULL) = (replaced_by IS NULL)),
    CONSTRAINT session_refresh_tokens_not_self_replaced CHECK (replaced_by IS NULL OR replaced_by <> id),
    CONSTRAINT session_refresh_tokens_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT session_refresh_tokens_expires_at_range CHECK (
        expires_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND expires_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT session_refresh_tokens_used_at_range CHECK (
        used_at IS NULL OR (
            used_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND used_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    )
);

CREATE INDEX session_refresh_tokens_session_id_idx ON session_refresh_tokens (session_id);
CREATE INDEX session_refresh_tokens_user_id_idx ON session_refresh_tokens (user_id);
