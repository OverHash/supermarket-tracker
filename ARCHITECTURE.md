# Project Architecture

### Countdown API

Countdown API endpoint can be found at https://www.countdown.co.nz/api/v1/

The countdown API requires an API key for all requests, which can be found in the base HTML at https://www.countdown.co.nz/

### Database

Running a Postgres instance with the following tables:

```
CREATE TYPE supermarket AS ENUM ('Countdown', 'New World');

countdown_stores
----------------
CREATE TABLE countdown_stores (
	id INTEGER PRIMARY KEY
	name VARCHAR(255) NOT NULL,
)

stores
----------------
CREATE TABLE stores (
	id SERIAL PRIMARY KEY,
	supermarket supermarket NOT NULL,
	countdown_store_id INTEGER UNIQUE,

	CONSTRAINT fk_countdown_id
		FOREIGN KEY(countdown_store_id)
			REFERENCES countdown_stores(id),

	CONSTRAINT chk_supermarket_type
		CHECK (
			(supermarket = 'Countdown' AND countdown_store_id IS NOT NULL)
			-- in the future, we add an OR to this to check new world store id
		)
)

countdown_products
------------------
CREATE TABLE countdown_products (
	id SERIAL PRIMARY KEY,
	name VARCHAR(255) NOT NULL,
	barcode VARCHAR(13) NOT NULL,
	sku VARCHAR(10) NOT NULL UNIQUE
)

products
--------
CREATE TABLE products (
	id SERIAL PRIMARY KEY,
	countdown_id INT,

	CONSTRAINT fk_countdown_product
		FOREIGN KEY(countdown_id)
			REFERENCES countdown_products(id)
)

prices
------
CREATE TABLE prices (
	id SERIAL PRIMARY KEY,
	product_id INTEGER NOT NULL,
	time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	cost_in_cents INTEGER NOT NULL,
	store_id INTEGER NOT NULL,

	CONSTRAINT fk_product
		FOREIGN KEY(product_id)
			REFERENCES products(id)

	CONSTRAINT fk_store_id
		FOREIGN KEY(store_id)
			REFERENCES stores(id)
)
```
