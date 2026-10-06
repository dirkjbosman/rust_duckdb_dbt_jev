-- Aggregate feedback counts and average rating per predicted category.
-- Equivalent plain SQL:
--   SELECT jev_classify(comment) AS category, count(*), avg(rating) FROM ...
select
    predicted_category                  as category,
    count(*)                            as n,
    round(avg(rating), 2)               as avg_rating,
    round(100.0 * sum(case when correct then 1 else 0 end) / count(*), 1) as accuracy_pct
from {{ ref('survey_classified') }}
group by 1
order by n desc
