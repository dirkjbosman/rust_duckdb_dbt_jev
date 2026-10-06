-- Overall accuracy of jev_classify() vs ground-truth labels.
-- Equivalent plain SQL:
--   SELECT round(100.0 * sum(correct) / count(*), 2) AS accuracy_pct FROM ...
select
    count(*)                                                        as total_rows,
    sum(case when correct then 1 else 0 end)                       as correct_count,
    round(
        100.0 * sum(case when correct then 1 else 0 end) / count(*),
    2)                                                              as accuracy_pct
from {{ ref('survey_classified') }}
