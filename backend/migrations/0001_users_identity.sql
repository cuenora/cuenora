-- Apply through a transactional migration runner. No authentication API exists yet.
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_original TEXT NOT NULL,
    email_address TEXT NOT NULL,
    email_key TEXT COLLATE "C" GENERATED ALWAYS AS (lower(email_address COLLATE "C")) STORED NOT NULL,
    email_verified_at TIMESTAMPTZ(6),
    profile_time_zone TEXT,
    created_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    CONSTRAINT users_email_key_unique UNIQUE (email_key),
    CONSTRAINT users_email_original_storage CHECK (
        octet_length(email_original) BETWEEN 3 AND 1024
        AND email_original COLLATE "C" !~ '[[:cntrl:]]'
        AND position('@' IN email_original) > 1
        AND split_part(email_original, '@', -1) <> ''
    ),
    -- This is a storage guard, not an RFC email parser or an IDNA converter.
    CONSTRAINT users_email_transport_storage CHECK (
        octet_length(email_address) BETWEEN 3 AND 254
        AND octet_length(email_address) = char_length(email_address)
        AND email_address COLLATE "C" !~ '[[:cntrl:]]'
        AND position('@' IN email_address) > 1
        AND split_part(email_address, '@', -1) <> ''
        AND split_part(email_address, '@', -1) = lower(split_part(email_address, '@', -1) COLLATE "C")
        AND char_length(email_address) - char_length(split_part(email_address, '@', -1)) - 1 BETWEEN 1 AND 64
    ),
    CONSTRAINT users_email_local_part_preserved CHECK (
        left(email_original, char_length(email_original) - char_length(split_part(email_original, '@', -1)) - 1)
        = left(email_address, char_length(email_address) - char_length(split_part(email_address, '@', -1)) - 1)
    ),
    CONSTRAINT users_profile_zone_storage CHECK (
        profile_time_zone IS NULL OR (
            octet_length(profile_time_zone) BETWEEN 1 AND 255
            AND profile_time_zone COLLATE "C" !~ '[[:space:][:cntrl:]]'
        )
    ),
    CONSTRAINT users_created_at_range CHECK (
        created_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND created_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT users_updated_at_range CHECK (
        updated_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND updated_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ),
    CONSTRAINT users_email_verified_at_range CHECK (
        email_verified_at IS NULL OR (
            email_verified_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
            AND email_verified_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
        )
    )
);

CREATE TABLE password_credentials (
    user_id UUID PRIMARY KEY REFERENCES users (id) ON DELETE RESTRICT,
    password_hash TEXT NOT NULL,
    password_changed_at TIMESTAMPTZ(6) NOT NULL DEFAULT statement_timestamp(),
    -- Full PHC parsing, hash strength and verification belong to P04.03.
    CONSTRAINT password_credentials_hash_storage CHECK (
        octet_length(password_hash) BETWEEN 11 AND 1024
        AND password_hash LIKE '$argon2id$%'
        AND password_hash COLLATE "C" !~ '[[:cntrl:]]'
    ),
    CONSTRAINT password_credentials_changed_at_range CHECK (
        password_changed_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND password_changed_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    )
);
