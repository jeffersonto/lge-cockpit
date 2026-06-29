-- Convert injection_phase from plain string to JSON array
-- e.g. "planning" -> '["planning"]'
UPDATE task_attachments
SET injection_phase = '["' || injection_phase || '"]'
WHERE injection_phase NOT LIKE '[%';
