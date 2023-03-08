ALTER TABLE reservations
DROP COLUMN start_time;

ALTER TABLE reservations
ADD COLUMN start_time Timestamptz; 