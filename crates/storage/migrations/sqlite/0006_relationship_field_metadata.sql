ALTER TABLE story_extraction_values
    ADD COLUMN related_character TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN relationship_type TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN relationship_label TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN relationship_direction TEXT;
