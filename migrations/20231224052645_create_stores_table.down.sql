-- Revert creating stores table

ALTER TABLE prices
	DROP COLUMN store_id,
	ADD COLUMN supermarket VARCHAR(255);
UPDATE prices
	SET supermarket = 'countdown';
ALTER TABLE prices
	ALTER COLUMN supermarket SET NOT NULL;

DROP TABLE stores;
DROP TABLE countdown_stores;
DROP TYPE supermarket;
