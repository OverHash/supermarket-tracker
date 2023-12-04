-- Revert creating database tables
-- We revert in the same order, to avoid foreign key complications
DROP TABLE prices;
DROP TABLE products;
DROP TABLE countdown_products;
