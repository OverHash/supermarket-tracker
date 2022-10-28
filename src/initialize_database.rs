use std::error::Error;

use sqlx::{Pool, Postgres};

pub async fn initialize_database(conn: &Pool<Postgres>) -> Result<(), Box<dyn Error>> {
    // create the supermarket table
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS countdown_products (
		id INTEGER,
		sku VARCHAR(10) NOT NULL,
		barcode VARCHAR(13) NOT NULL,

		PRIMARY KEY(id)
	)"#,
    )
    .execute(conn)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS products (
		id INTEGER,
		countdown_id INT,
	
		PRIMARY KEY(id),
		CONSTRAINT fk_countdown_product
			FOREIGN KEY(countdown_id)
				REFERENCES countdown_products(id)	
	)"#,
    )
    .execute(conn)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS prices (
		id INTEGER,
		product_id INTEGER NOT NULL,
		time TIMESTAMPTZ NOT NULL,
		cost_in_cents INTEGER NOT NULL,
		supermarket VARCHAR(255) NOT NULL,
	
		PRIMARY KEY(id),
		CONSTRAINT fk_product
			FOREIGN KEY(product_id)
				REFERENCES products(id)
	)"#,
    )
    .execute(conn)
    .await?;

    Ok(())
}
