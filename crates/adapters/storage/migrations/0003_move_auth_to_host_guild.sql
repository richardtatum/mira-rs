ALTER TABLE host_guild ADD COLUMN auth_header TEXT;

UPDATE host_guild
SET auth_header = (SELECT h.auth_header FROM host h WHERE h.id = host_guild.host_id);

ALTER TABLE host DROP COLUMN auth_header;
