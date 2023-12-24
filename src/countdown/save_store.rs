use error_stack::Result;
use sqlx::PgPool;

/// Saves a Countdown store into the database.
///
/// If the store already exists, returns the store id.
#[tracing::instrument(
	name = "save_store",
	level = "debug",
	skip_all,
	fields(
		%id,
		%name
	)
)]
pub async fn save_store(pool: &PgPool, id: i32, name: String) -> Result<i32, sqlx::Error> {
    let countdown_store_id = sqlx::query!(
        r#"INSERT INTO countdown_stores (
			id, name ) VALUES (
				$1, $2
			) ON CONFLICT DO NOTHING
			RETURNING id"#,
        id,
        name,
    )
    .fetch_optional(pool)
    .await?;

    if countdown_store_id.is_some() {
        // insert and return new store id
        let id = sqlx::query!(
            "INSERT INTO stores (
supermarket, countdown_store_id
) VALUES (
'Countdown', $1
) RETURNING id",
            id
        )
        .fetch_one(pool)
        .await?;
        return Ok(id.id);
    }

    // return the existing store id
    let id = sqlx::query!(
        r#"SELECT id FROM stores
WHERE supermarket = 'Countdown' AND countdown_store_id = $1"#,
        id
    )
    .fetch_one(pool)
    .await?;
    return Ok(id.id);
}
