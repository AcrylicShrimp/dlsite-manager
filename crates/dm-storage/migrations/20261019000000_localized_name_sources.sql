-- Preserve legacy scalar names only when the localized map has no usable text.
UPDATE works
SET title_json = CASE
    WHEN trim(title) <> '' THEN json_object('und', title)
    ELSE '{}'
END
WHERE NOT EXISTS (
    SELECT 1 FROM json_each(
        CASE WHEN json_valid(title_json)
            THEN CASE WHEN json_type(title_json) = 'object' THEN title_json ELSE '{}' END
            ELSE '{}'
        END
    ) AS name
    WHERE name.type = 'text' AND trim(name.value) <> ''
);

UPDATE works
SET maker_json = CASE
    WHEN trim(COALESCE(maker_name, '')) <> '' THEN json_object('und', maker_name)
    ELSE NULL
END
WHERE NOT EXISTS (
    SELECT 1 FROM json_each(
        CASE WHEN json_valid(maker_json)
            THEN CASE WHEN json_type(maker_json) = 'object' THEN maker_json ELSE '{}' END
            ELSE '{}'
        END
    ) AS name
    WHERE name.type = 'text' AND trim(name.value) <> ''
);

DROP INDEX works_title_idx;
DROP INDEX works_maker_name_idx;
ALTER TABLE works DROP COLUMN title;
ALTER TABLE works DROP COLUMN maker_name;
