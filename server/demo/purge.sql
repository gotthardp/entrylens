-- SPDX-License-Identifier: MPL-2.0
--
-- Deletes all passages (the demo data) and the occupancy and daily values computed from them,
-- and the sensors' reported status, e.g. before going live or before re-running passages.sql.
-- Rooms and doorways are kept.
--
--   sqlite3 entrylens.db < purge.sql

BEGIN;
DELETE FROM daily;
DELETE FROM occupancy;
DELETE FROM passages;
UPDATE doorways SET fault = 0, voltage_mv = NULL, last_report = NULL;
COMMIT;
