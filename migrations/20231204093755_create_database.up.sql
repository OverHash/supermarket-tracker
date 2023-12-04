-- Create `countdown_products` table
CREATE TABLE countdown_products (
	id SERIAL PRIMARY KEY,
	name VARCHAR(255) NOT NULL,
	barcode VARCHAR(13) NOT NULL,
	sku VARCHAR(10) NOT NULL UNIQUE
);

-- Create `products` table
CREATE TABLE products (
	id SERIAL PRIMARY KEY,
	countdown_id INT,

	CONSTRAINT fk_countdown_product
		FOREIGN KEY(countdown_id)
			REFERENCES countdown_products(id)
);

-- Create `prices` table
CREATE TABLE prices (
	id SERIAL PRIMARY KEY,
	product_id INTEGER NOT NULL,
	time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	cost_in_cents INTEGER NOT NULL,
	supermarket VARCHAR(255) NOT NULL,

	CONSTRAINT fk_product
		FOREIGN KEY(product_id)
			REFERENCES products(id)
);
