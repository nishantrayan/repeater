-- 1. Add the column with NOT NULL + default
ALTER TABLE cards
ADD COLUMN review_stage TEXT NOT NULL DEFAULT 'new';

-- 2. Update existing rows where review_count > 0
UPDATE cards
SET review_stage = 'review'
WHERE review_count > 0;
