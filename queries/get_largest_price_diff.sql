WITH price_history AS (
	SELECT
		product_id,
		name,
		SUM(CASE WHEN rank = 1 THEN cost_in_cents ELSE null END) today, 
		SUM(CASE WHEN rank = 2 THEN cost_in_cents ELSE null END) yesterday,
		SUM(CASE WHEN rank = 1 THEN cost_in_cents ELSE 0 END) -
		SUM(CASE WHEN rank = 2 THEN cost_in_cents ELSE 0 END) as diff
	FROM (SELECT product_id, cost_in_cents, time, name, rank()
		OVER (PARTITION BY product_id ORDER BY time DESC) AS rank
		FROM prices
			JOIN products ON products.id = prices.product_id
			JOIN countdown_products ON countdown_products.id = products.id
		WHERE time BETWEEN current_date - 1 AND current_date + 1
	) AS p
	WHERE rank IN (1,2)
	GROUP BY (product_id, name)
)
SELECT * FROM price_history WHERE (CASE WHEN today IS null OR yesterday IS null then 'NULL' else 'VALID' end) != 'NULL' AND diff <> 0
ORDER BY diff DESC;