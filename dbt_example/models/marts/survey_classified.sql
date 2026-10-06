-- Classify every feedback comment using the JEV extension.
-- Equivalent plain SQL:
--   SELECT comment, jev_classify(comment) AS predicted_category FROM ...
select
    feedback_id,
    user_id,
    rating,
    comment,
    jev_classify(comment)                              as predicted_category,
    true_category,
    jev_classify(comment) = true_category              as correct
from {{ ref('stg_survey_feedback') }}
