-- This file should undo anything in `up.sql`ALTER TABLE reservations
ALTER COLUMN start_time DATETIME; 
ALTER TABLE reservations
ALTER COLUMN start_time DATETIME; 