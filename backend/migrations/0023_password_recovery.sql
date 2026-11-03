CREATE TABLE password_recovery_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES password_credentials (user_id) ON DELETE RESTRICT,
    email TEXT COLLATE "C" NOT NULL,
    credential_revision BIGINT NOT NULL CHECK (credential_revision > 0),
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ(6) NOT NULL,
    consumed_at TIMESTAMPTZ(6),
    invalidated_at TIMESTAMPTZ(6),
    CONSTRAINT password_recovery_challenges_expiry CHECK (expires_at > created_at),
    CONSTRAINT password_recovery_challenges_terminal_state
        CHECK (consumed_at IS NULL OR invalidated_at IS NULL)
);

CREATE INDEX password_recovery_challenges_user_created_idx
    ON password_recovery_challenges (user_id, created_at DESC, id);
CREATE INDEX password_recovery_challenges_live_idx
    ON password_recovery_challenges (user_id, expires_at, id)
    WHERE consumed_at IS NULL AND invalidated_at IS NULL;

-- This outbox deliberately contains no recovery token, password, or token digest.
CREATE TABLE password_reset_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES password_credentials (user_id) ON DELETE RESTRICT,
    email TEXT COLLATE "C" NOT NULL,
    idempotency_key TEXT COLLATE "C" NOT NULL UNIQUE,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    available_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    lease_until TIMESTAMPTZ(6),
    attempts SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 5),
    delivered_at TIMESTAMPTZ(6),
    failed_at TIMESTAMPTZ(6),
    CONSTRAINT password_reset_notifications_terminal_state
        CHECK (delivered_at IS NULL OR failed_at IS NULL)
);

CREATE INDEX password_reset_notifications_ready_idx
    ON password_reset_notifications (available_at, created_at, id)
    WHERE delivered_at IS NULL AND failed_at IS NULL;
