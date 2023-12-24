-- We make our database more type-safe.
-- Previously, all collected rows are for a Countdown store in the North Island.
-- We now create a specific supermarket enum variant and store info
BEGIN;
	CREATE TYPE supermarket AS ENUM ('Countdown', 'New World');

	CREATE TABLE countdown_stores (
		id INTEGER PRIMARY KEY,
		name VARCHAR(255) NOT NULL
	);

	INSERT INTO countdown_stores (
		id, name
	) VALUES (
		2124460,
		'Countdown Birkenhead'
	) RETURNING id AS birkenhead_countdown_store_id;

	CREATE TABLE stores (
		id SERIAL PRIMARY KEY,
		supermarket supermarket NOT NULL,
		countdown_store_id INTEGER UNIQUE,

		CONSTRAINT fk_countdown_store_id
			FOREIGN KEY(countdown_store_id)
				REFERENCES countdown_stores(id),

		CONSTRAINT chk_supermarket_type
			CHECK (
				(supermarket = 'Countdown' AND countdown_store_id IS NOT NULL)
			)
	);

	INSERT INTO stores (
		supermarket, countdown_store_id
	) VALUES (
		'Countdown',
		2124460
	);

	ALTER TABLE prices
		DROP COLUMN supermarket,
		ADD COLUMN store_id INTEGER;
	
	UPDATE prices
	SET store_id = (
		SELECT id
		FROM stores
		WHERE countdown_store_id = 2124460
		LIMIT 1
	);
	
	ALTER TABLE prices
		ALTER COLUMN store_id SET NOT NULL,
		ADD CONSTRAINT fk_store_id
			FOREIGN KEY(store_id)
				REFERENCES stores(id);
COMMIT;
