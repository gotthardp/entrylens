-- SPDX-License-Identifier: MPL-2.0
--
-- Demo layout: rooms A, B and C (occupancy thresholds 50, 12 and none), doorways from the outside
-- corridor (the hallway) into each room, from A into B and from C into B, and the sensors'
-- DevEUIs. Run it once the server has created the tables, then passages.sql.
--
--   sqlite3 entrylens.db < layout.sql

.bail on
BEGIN;

INSERT INTO rooms (name, threshold) VALUES ('A', 50), ('B', 12), ('C', NULL);

INSERT INTO doorways (inside_room, outside_room)
SELECT id, NULL FROM rooms WHERE name IN ('A', 'B', 'C') ORDER BY name;

INSERT INTO doorways (inside_room, outside_room)
SELECT b.id, a.id FROM rooms a, rooms b WHERE a.name = 'A' AND b.name = 'B';

INSERT INTO doorways (inside_room, outside_room)
SELECT b.id, c.id FROM rooms c, rooms b WHERE c.name = 'C' AND b.name = 'B';

-- The sensors' DevEUIs, in the order the doorways were created.
UPDATE doorways SET dev_eui = printf('AC1F09FFFE0A%04X', id);

COMMIT;
