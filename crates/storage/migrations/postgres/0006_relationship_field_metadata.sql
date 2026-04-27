ALTER TABLE story_extraction_values
    ADD COLUMN IF NOT EXISTS related_character TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN IF NOT EXISTS relationship_type TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN IF NOT EXISTS relationship_label TEXT;

ALTER TABLE story_extraction_values
    ADD COLUMN IF NOT EXISTS relationship_direction TEXT;
