with price_history as (
	SELECT
		product_id,
		SUM(CASE WHEN rank = 1 THEN cost_in_cents ELSE null END) today, 
		SUM(CASE WHEN rank = 2 THEN cost_in_cents ELSE null END) yesterday,
		SUM(CASE WHEN rank = 1 THEN cost_in_cents ELSE 0 END) -
		SUM(CASE WHEN rank = 2 THEN cost_in_cents ELSE 0 END) as diff
	FROM (SELECT product_id, cost_in_cents, time, rank()
		OVER (PARTITION BY product_id ORDER BY time DESC) AS rank
		FROM prices WHERE time BETWEEN current_date - 1 AND current_date + 1
	) AS p
	WHERE rank IN (1,2)
	GROUP BY product_id
)
SELECT * FROM price_history WHERE (CASE WHEN today IS null OR yesterday IS null then 'NULL' else 'VALID' end) != 'NULL'
ORDER BY diff DESC;