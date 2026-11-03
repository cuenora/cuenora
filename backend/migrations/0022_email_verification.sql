ALTER TABLE password_credentials
    ADD COLUMN credential_revision BIGINT NOT NULL DEFAULT 1,
    ADD CONSTRAINT password_credentials_revision_positive CHECK (credential_revision > 0);

CREATE TABLE email_verification_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES password_credentials (user_id) ON DELETE RESTRICT,
    email TEXT COLLATE "C" NOT NULL,
    credential_revision BIGINT NOT NULL CHECK (credential_revision > 0),
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ(6) NOT NULL,
    consumed_at TIMESTAMPTZ(6),
    invalidated_at TIMESTAMPTZ(6),
    CONSTRAINT email_verification_challenges_expiry CHECK (expires_at > created_at),
    CONSTRAINT email_verification_challenges_terminal_state
        CHECK (consumed_at IS NULL OR invalidated_at IS NULL),
    CONSTRAINT email_verification_challenges_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT email_verification_challenges_expires_at_range CHECK (
        expires_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND expires_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT email_verification_challenges_consumed_at_range CHECK (
        consumed_at IS NULL OR (
            consumed_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND consumed_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    ),
    CONSTRAINT email_verification_challenges_invalidated_at_range CHECK (
        invalidated_at IS NULL OR (
            invalidated_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND invalidated_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    )
);

CREATE INDEX email_verification_challenges_user_created_idx
    ON email_verification_challenges (user_id, created_at DESC, id);
CREATE INDEX email_verification_challenges_live_idx
    ON email_verification_challenges (user_id, expires_at, id)
    WHERE consumed_at IS NULL AND invalidated_at IS NULL;
