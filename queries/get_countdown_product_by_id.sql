SELECT * FROM countdown_products
	JOIN products ON products.countdown_id = countdown_products.id
	JOIN prices ON prices.product_id = products.id
WHERE product_id = '$1';