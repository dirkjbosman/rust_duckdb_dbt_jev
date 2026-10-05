use quack_rs::prelude::*;
use quack_rs::entry_point;
use quack_rs::vector::string::read_duck_string;
use libduckdb_sys::{
    duckdb_data_chunk, duckdb_data_chunk_get_size, duckdb_data_chunk_get_vector,
    duckdb_function_info, duckdb_vector, duckdb_vector_get_data,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Survey feedback categories for Kleinanzeigen
// ---------------------------------------------------------------------------

const CATEGORIES: &[(&str, &str)] = &[
    (
        "Fraud or Scam Risk",
        "User mentions fake listings, payment redirection outside the platform, or phishing.",
    ),
    (
        "Shipping / Delivery Problem",
        "Issues with DHL, Hermes, missing packages, delayed shipping, or wrong shipping costs.",
    ),
    (
        "App Technical Bug",
        "App crashes, error codes (e.g. 500), UI freezing, broken chat, or login problems.",
    ),
    (
        "Customer Service Complaint",
        "Support response too slow, account blocked unfairly, or unhelpful automated answers.",
    ),
    (
        "General Praise / Positive",
        "User is happy with the platform, fast sale, smooth transaction, or great buyer/seller.",
    ),
];

// ---------------------------------------------------------------------------
// HTTP client — shared across calls, built once
// ---------------------------------------------------------------------------

static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::blocking::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

fn jev_base_url() -> String {
    std::env::var("JEV_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

fn jev_model() -> String {
    std::env::var("JEV_MODEL").unwrap_or_else(|_| "nimble".to_string())
}

// ---------------------------------------------------------------------------
// Ollama /v1/systemone JEV request/response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JevRequest {
    model: String,
    state: HashMap<String, String>,
    questions: HashMap<String, JevQuestion>,
}

#[derive(Serialize)]
struct JevQuestion {
    #[serde(rename = "type")]
    kind: &'static str,
    instructions: &'static str,
    criteria: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct JevResponse {
    answers: HashMap<String, JevAnswer>,
}

#[derive(Deserialize, Debug)]
struct JevAnswer {
    choice: Option<String>,
    probabilities: Option<HashMap<String, f64>>,
}

// ---------------------------------------------------------------------------
// Core Ollama call
// ---------------------------------------------------------------------------

fn call_ollama(text: &str) -> Result<JevAnswer, Box<dyn std::error::Error>> {
    let mut state = HashMap::new();
    state.insert("text".to_string(), text.to_string());

    let criteria: HashMap<String, String> = CATEGORIES
        .iter()
        .map(|(name, desc)| (name.to_string(), desc.to_string()))
        .collect();

    let mut questions = HashMap::new();
    questions.insert(
        "category".to_string(),
        JevQuestion {
            kind: "choice",
            instructions: "Classify this user feedback into exactly one of the provided categories.",
            criteria,
        },
    );

    let request = JevRequest {
        model: jev_model(),
        state,
        questions,
    };

    let url = format!("{}/v1/systemone", jev_base_url());
    let response = http_client().post(&url).json(&request).send()?;

    if !response.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", response.status()).into());
    }

    let mut parsed: JevResponse = response.json()?;
    parsed
        .answers
        .remove("category")
        .ok_or_else(|| "no 'category' key in JEV response".into())
}

// ---------------------------------------------------------------------------
// Keyword fallback — used when Ollama is unreachable
// ---------------------------------------------------------------------------

fn keyword_fallback(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("scam")
        || t.contains("whatsapp")
        || t.contains("fake")
        || t.contains("phish")
        || t.contains("redirect")
        || t.contains("suspicious")
    {
        "Fraud or Scam Risk"
    } else if t.contains("ship")
        || t.contains("package")
        || t.contains("hermes")
        || t.contains("dhl")
        || t.contains("deliver")
        || t.contains("tracking")
    {
        "Shipping / Delivery Problem"
    } else if t.contains("bug")
        || t.contains("crash")
        || t.contains("error")
        || t.contains("broken")
        || t.contains("freeze")
        || t.contains("500")
        || t.contains("notification")
    {
        "App Technical Bug"
    } else if t.contains("support")
        || t.contains("blocked")
        || t.contains("service")
        || t.contains("dispute")
        || t.contains("reply")
    {
        "Customer Service Complaint"
    } else {
        "General Praise / Positive"
    }
}

// ---------------------------------------------------------------------------
// DuckDB scalar: jev_classify(comment VARCHAR) -> VARCHAR
//
// Returns the top predicted category for the input feedback text.
// Falls back to keyword matching if Ollama is not reachable.
// ---------------------------------------------------------------------------

unsafe extern "C" fn jev_classify_fn(
    _info: duckdb_function_info,
    chunk: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let size = unsafe { duckdb_data_chunk_get_size(chunk) } as usize;
    let input_vec = unsafe { duckdb_data_chunk_get_vector(chunk, 0) };
    let input_data = unsafe { duckdb_vector_get_data(input_vec) } as *const u8;
    let mut writer = unsafe { VectorWriter::new(output) };

    for i in 0..size {
        let text = unsafe { read_duck_string(input_data, i) };
        let category = match call_ollama(text) {
            Ok(answer) => answer
                .choice
                .unwrap_or_else(|| keyword_fallback(text).to_string()),
            Err(_) => keyword_fallback(text).to_string(),
        };
        unsafe { writer.write_varchar(i, &category) };
    }
}

// ---------------------------------------------------------------------------
// DuckDB scalar: jev_classify_prob(comment VARCHAR, category VARCHAR) -> DOUBLE
//
// Returns the probability (0.0–1.0) that the comment belongs to the given
// category. Useful for confidence thresholding or ranking.
// ---------------------------------------------------------------------------

unsafe extern "C" fn jev_classify_prob_fn(
    _info: duckdb_function_info,
    chunk: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let size = unsafe { duckdb_data_chunk_get_size(chunk) } as usize;
    let text_vec = unsafe { duckdb_data_chunk_get_vector(chunk, 0) };
    let cat_vec = unsafe { duckdb_data_chunk_get_vector(chunk, 1) };
    let text_data = unsafe { duckdb_vector_get_data(text_vec) } as *const u8;
    let cat_data = unsafe { duckdb_vector_get_data(cat_vec) } as *const u8;
    let mut writer = unsafe { VectorWriter::new(output) };

    for i in 0..size {
        let text = unsafe { read_duck_string(text_data, i) };
        let category = unsafe { read_duck_string(cat_data, i) };
        let prob = match call_ollama(text) {
            Ok(answer) => answer
                .probabilities
                .as_ref()
                .and_then(|p| p.get(category).copied())
                .unwrap_or(0.0),
            Err(_) => {
                // Fallback: return 1.0 if keyword logic agrees, else 0.0
                if keyword_fallback(text) == category {
                    1.0_f64
                } else {
                    0.0_f64
                }
            }
        };
        unsafe { writer.write_f64(i, prob) };
    }
}

// ---------------------------------------------------------------------------
// Extension entry point — registers both functions with DuckDB
// ---------------------------------------------------------------------------

entry_point!(jev_init_c_api, |con| {
    ScalarFunctionBuilder::new("jev_classify")
        .param(TypeId::Varchar)
        .returns(TypeId::Varchar)
        .function(jev_classify_fn)
        .register(con)?;

    ScalarFunctionBuilder::new("jev_classify_prob")
        .param(TypeId::Varchar)
        .param(TypeId::Varchar)
        .returns(TypeId::Double)
        .function(jev_classify_prob_fn)
        .register(con)?;

    Ok(())
});
