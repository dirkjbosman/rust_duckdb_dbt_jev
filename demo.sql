-- Load the JEV extension
LOAD './jev.duckdb_extension';

-- Classify all comments in the survey batch
SELECT
    feedback_id,
    rating,
    comment,
    jev_classify(comment)                              AS predicted_category,
    true_category,
    jev_classify(comment) = true_category              AS correct
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
LIMIT 20;

-- Aggregate: how many feedback items per predicted category?
SELECT
    jev_classify(comment) AS category,
    count(*)              AS n,
    round(avg(rating), 2) AS avg_rating
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
GROUP BY 1
ORDER BY n DESC;

-- Accuracy check against ground-truth labels
SELECT
    round(
        100.0 * sum(CASE WHEN jev_classify(comment) = true_category THEN 1 ELSE 0 END)
        / count(*),
    2) AS accuracy_pct
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv');

-- Confidence score for a specific category
SELECT
    feedback_id,
    comment,
    jev_classify_prob(comment, 'Fraud or Scam Risk') AS fraud_prob
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
WHERE rating <= 2
ORDER BY fraud_prob DESC
LIMIT 10;
