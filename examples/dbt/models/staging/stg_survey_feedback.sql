-- Staging model: clean types from the raw seed table
select
    feedback_id,
    user_id,
    cast(rating as integer) as rating,
    comment,
    true_category
from {{ ref('kleinanzeigen_surveys') }}
where comment is not null
  and trim(comment) != ''
