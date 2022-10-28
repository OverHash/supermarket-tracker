# Project Architecture

### Countdown API

Countdown API endpoint can be found at https://www.countdown.co.nz/api/v1/

The countdown API requires an API key for all requests, which can be found in the base HTML at https://www.countdown.co.nz/

### Database

Running a Postgres instance with the following tables:

```
countdown_products
------------------
CREATE TABLE countdown_products (
	id INTEGER,
	sku VARCHAR(10) NOT NULL,
	barcode VARCHAR(13) NOT NULL,

	PRIMARY KEY(id)
)

products
--------
CREATE TABLE products (
	id INTEGER,
	countdown_id INT,

	PRIMARY KEY(id),
	CONSTRAINT fk_countdown_product
		FOREIGN KEY(countdown_id)
			REFERENCES countdown_products(id)
)

prices
------
CREATE TABLE PRICES (
	id INTEGER,
	product_id INTEGER NOT NULL,
	time TIMESTAMPTZ NOT NULL,
	cost_in_cents INTEGER NOT NULL,
	supermarket VARCHAR(255) NOT NULL,

	PRIMARY KEY(id),
	CONSTRAINT fk_product
		FOREIGN KEY(product_id)
			REFERENCES products(id)
)
```
