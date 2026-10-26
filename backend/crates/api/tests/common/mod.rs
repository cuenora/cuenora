use sqlx::PgPool;
use std::env;

pub async fn pool() -> PgPool {
    assert_eq!(env::var("CUENORA_TEST_DISPOSABLE").as_deref(), Ok("1"));
    let port: u16 = env::var("CUENORA_TEST_PGPORT").unwrap().parse().unwrap();
    let password = env::var("CUENORA_TEST_PGPASSWORD").unwrap();
    // The Compose harness generates token_urlsafe credentials, not arbitrary URLs.
    assert!(
        password
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    );
    let url = format!("postgres://cuenora:{password}@127.0.0.1:{port}/cuenora");
    cuenora_api::config::pool_from_url(Some(&url))
        .unwrap()
        .unwrap()
}

pub async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_array((SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u), (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c))::text)")
        .fetch_one(pool).await.unwrap()
}
