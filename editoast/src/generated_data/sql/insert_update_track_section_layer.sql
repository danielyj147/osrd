INSERT INTO infra_layer_track_section (obj_id, infra_id, geographic)
SELECT obj_id,
    $1,
    ST_Transform(ST_GeomFromGeoJSON(data->'geo'), 3857)
FROM infra_object_track_section
WHERE infra_id = $1
    AND obj_id = ANY($2) ON CONFLICT (infra_id, obj_id) DO
UPDATE
SET geographic = EXCLUDED.geographic
