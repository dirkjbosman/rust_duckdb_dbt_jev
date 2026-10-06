# Jev Rust Extension for Duck-DB & dbt

A DuckDB loadable extension written in Rust that classifies open-ended survey feedback using [Ollama's JEV-style decision models](https://ollama.com/blog/ollama-now-supports-jev-style-decision-models) — entirely free and local, no API key required.

Built for Kleinanzeigen survey data but works for any text classification task.

---

## Accuracy

Tested on 50 labelled Kleinanzeigen survey responses across 5 categories:

| Metric | Result |
|---|---|
| Accuracy | **98%** (49/50 correct) |
| Model | `nimble` (9.5 GB, Bespoke Labs) |
| Inference | ~6s per row on CPU |

---

## What it does

Adds two SQL functions to DuckDB:

| Function | Description |
|---|---|
| `jev_classify(comment)` | Returns the best-fit category for a feedback comment |
| `jev_classify_prob(comment, category)` | Returns a probability (0.0–1.0) for a specific category |

Categories used:
- `Fraud or Scam Risk`
- `Shipping / Delivery Problem`
- `App Technical Bug`
- `Customer Service Complaint`
- `General Praise / Positive`

---

## Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Ollama](https://ollama.com/download) v0.35+
- [DuckDB CLI](https://duckdb.org/docs/installation)
- [uv](https://docs.astral.sh/uv/getting-started/installation/) *(for the dbt example only)*

---

## Setup

### 1. Install or update Ollama

```bash
ollama --version   # must be v0.35+
```

If below v0.35:
```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### 2. Start Ollama and pull nimble

```bash
ollama serve &
ollama pull nimble
```

`nimble` is a 9.5 GB JEV decision model. For a smaller/faster option use `tev1:0.8b` — see [Configuration](#configuration).

### 3. Build the extension

```bash
cargo build --release
```

### 4. Package as a DuckDB extension

Install the metadata tool (one-time):
```bash
cargo install quack-rs --bin append_metadata
```

Then package:
```bash
make package
```

Produces `jev.duckdb_extension` in the project root. The Makefile auto-detects your platform (`osx_amd64`, `osx_arm64`, `linux_amd64`, `linux_arm64`).

---

## Examples

### Plain SQL (`examples/stdsql/`)

DuckDB must be started with `-unsigned` to load locally built extensions:

```bash
duckdb -unsigned
```

```sql
LOAD './jev.duckdb_extension';

-- Classify a single comment
SELECT jev_classify('app keeps crashing on iOS');
-- → App Technical Bug

-- Classify all survey comments
SELECT
    feedback_id,
    comment,
    jev_classify(comment)             AS predicted_category,
    true_category
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv');

-- Aggregate by category
SELECT
    jev_classify(comment)             AS category,
    count(*)                          AS n,
    round(avg(rating), 2)             AS avg_rating
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
GROUP BY 1
ORDER BY n DESC;

-- Accuracy vs ground truth (reproduces the 98% result)
SELECT
    round(
        100.0 * sum(CASE WHEN jev_classify(comment) = true_category THEN 1 ELSE 0 END)
        / count(*),
    2) AS accuracy_pct,
    count(*) AS total_rows
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv');

-- Probability score for a specific category
SELECT
    comment,
    jev_classify_prob(comment, 'Fraud or Scam Risk') AS fraud_prob
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
WHERE rating <= 2
ORDER BY fraud_prob DESC;
```

All queries are in `examples/stdsql/demo.sql`.

---

### dbt (`examples/dbt/`)

Uses the [`dbt-duckdb`](https://github.com/duckdb/dbt-duckdb) adapter.

#### Install dependencies with uv

```bash
cd examples/dbt
uv sync
```

#### Run

```bash
uv run dbt seed --profiles-dir .
uv run dbt run  --profiles-dir .
```

Set `JEV_EXTENSION_PATH` if your extension file is not at `../../jev.duckdb_extension`:

```bash
export JEV_EXTENSION_PATH=/absolute/path/to/jev.duckdb_extension
uv run dbt run --profiles-dir .
```

#### Models

| Model | What it does |
|---|---|
| `staging/stg_survey_feedback` | Clean types from the raw seed |
| `marts/survey_classified` | `jev_classify(comment)` per row + correctness flag |
| `marts/survey_category_summary` | Count + avg rating per category |
| `marts/survey_accuracy` | Overall accuracy % vs ground-truth labels |

Models run in dependency order automatically (`stg → classified → summary / accuracy`).

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `JEV_MODEL` | `nimble` | Ollama model to use |
| `JEV_BASE_URL` | `http://localhost:11434` | Ollama base URL |

```bash
export JEV_MODEL=tev1:0.8b          # smaller/faster model
export JEV_BASE_URL=http://host:11434
duckdb -unsigned
```

---

## Fallback behaviour

If Ollama is not reachable, the extension falls back to keyword matching so queries never hard-fail. Classification quality will be lower but every row still returns a result.

---

## Project structure

```
src/lib.rs                            — extension source (Rust)
Cargo.toml                            — Rust dependencies
Makefile                              — build + package shortcut
kleinanzeigen_surveys_csv_survey_batch_013.csv  — sample data (synthetic)
examples/
  stdsql/
    demo.sql                          — plain DuckDB SQL queries
  dbt/
    pyproject.toml                    — Python dependencies (uv)
    profiles.yml                      — DuckDB connection config
    dbt_project.yml                   — project config + extension load hook
    seeds/kleinanzeigen_surveys.csv   — survey data for dbt seed
    models/staging/
      stg_survey_feedback.sql
    models/marts/
      survey_classified.sql
      survey_category_summary.sql
      survey_accuracy.sql
```
