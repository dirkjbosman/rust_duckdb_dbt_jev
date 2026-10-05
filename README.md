# duckdb-jev-rs

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

To reproduce:

```bash
duckdb -unsigned -c "
LOAD './jev.duckdb_extension';
SELECT
    round(
        100.0 * sum(CASE WHEN jev_classify(comment) = true_category THEN 1 ELSE 0 END)
        / count(*),
    2) AS accuracy_pct,
    count(*) AS total_rows
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv');
"
```

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

---

## Setup

### 1. Install or update Ollama

Check your version:
```bash
ollama --version
```

If it is below v0.35, update it:
```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### 2. Start Ollama and pull the nimble model

```bash
ollama serve &
ollama pull nimble
```

`nimble` is a 9.5 GB JEV decision model by Bespoke Labs. If you want something smaller and faster, use `tev1:0.8b` (0.8 GB) instead — see [Configuration](#configuration).

### 3. Build the extension

```bash
cargo build --release
```

### 4. Package it as a DuckDB extension

Install the metadata tool (one-time):
```bash
cargo install quack-rs --bin append_metadata
```

Then package:
```bash
make package
```

This produces `jev.duckdb_extension` in the project root. The Makefile auto-detects your platform (`osx_amd64`, `osx_arm64`, `linux_amd64`, `linux_arm64`).

---

## Usage

DuckDB must be started with `-unsigned` to load locally built extensions:

```bash
duckdb -unsigned
```

Then in DuckDB:

```sql
LOAD './jev.duckdb_extension';

-- Classify a single comment
SELECT jev_classify('app keeps crashing on iOS');
-- → App Technical Bug

-- Classify all survey comments
SELECT
    feedback_id,
    comment,
    jev_classify(comment) AS predicted_category,
    true_category
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv');

-- Aggregate by category
SELECT
    jev_classify(comment) AS category,
    count(*)              AS n,
    round(avg(rating), 2) AS avg_rating
FROM read_csv('kleinanzeigen_surveys_csv_survey_batch_013.csv')
GROUP BY 1
ORDER BY n DESC;

-- Accuracy vs ground truth
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

All queries are also available in `demo.sql`.

---

## Configuration

Override the model or Ollama URL via environment variables before starting DuckDB:

```bash
# Use the smaller/faster 0.8B model
export JEV_MODEL=tev1:0.8b

# Point to a remote Ollama instance
export JEV_BASE_URL=http://my-server:11434

duckdb -unsigned
```

| Variable | Default | Description |
|---|---|---|
| `JEV_MODEL` | `nimble` | Ollama model to use |
| `JEV_BASE_URL` | `http://localhost:11434` | Ollama base URL |

---

## Fallback behaviour

If Ollama is not reachable, the extension falls back to keyword matching so queries never hard-fail. Classification quality will be lower but every row will still return a result.

---

## Project structure

```
src/lib.rs       — extension source (Rust)
Cargo.toml       — dependencies
Makefile         — build + package shortcut
demo.sql         — example DuckDB queries
kleinanzeigen_surveys_csv_survey_batch_013.csv  — sample data (synthetic)
```
