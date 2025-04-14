use crate::types::{Difficulty, OpenAIRequest, OpenAIMessage, OpenAIResponse}; // Use local types
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse,
};
use serde_json;

// --- Configuration (Consider moving to a config module or constants in lib.rs/state.rs) ---
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const OPENAI_MODEL: &str = "gpt-3.5-turbo"; // Or gpt-4o-mini etc.
const DARE_MAX_TOKENS: u32 = 60;
const HTTP_REQUEST_CYCLES: u128 = 70_000_000_000; // Adjust based on testing!

// --- API Key Handling ---

// Placeholder for securely getting API key
// WARNING: THIS IS INSECURE FOR PRODUCTION. DO NOT HARDCODE KEYS.
// Replace with a secure method like encrypted storage or configuration management.
fn get_openai_api_key() -> Result<String, String> {
    let key = "YOUR_OPENAI_API_KEY_HERE"; // <<<!!! REPLACE AND SECURE THIS !!!>>>
    if key == "YOUR_OPENAI_API_KEY_HERE" {
        ic_cdk::println!("WARNING: Using placeholder API key in llm.rs. Replace get_openai_api_key() with a secure method!");
        return Err("API Key is not configured securely. Update get_openai_api_key() in llm.rs.".to_string());
    }
    Ok(key.to_string())
}

// --- Core LLM Interaction Logic ---

// Fetches a dare from the LLM based on difficulty
pub async fn fetch_llm_dare(difficulty: Difficulty) -> Result<String, String> {
    let api_key = get_openai_api_key()?; // Propagate error if key not set

    // Construct Prompt
    let difficulty_str = format!("{:?}", difficulty).to_lowercase();
    let prompt = format!(
        "You are an assistant generating dares for an online community bot. Generate one short, fun, creative dare with '{}' difficulty. The dare should be actionable online or briefly in real life. IMPORTANT: Respond ONLY with the text of the dare itself, without any extra formatting, quotation marks, or preamble like 'Here is a dare:'.",
        difficulty_str
    );

     // Prepare Request Body
    let request_body = OpenAIRequest {
        model: OPENAI_MODEL,
        messages: vec![OpenAIMessage { role: "user", content: &prompt }],
        max_tokens: DARE_MAX_TOKENS,
        temperature: 0.8, // Adjust creativity
    };
    // Use map_err for better error context
    let request_body_json = serde_json::to_string(&request_body)
        .map_err(|e| format!("LLM Request Serialization Error: {}", e))?;
    let request_body_bytes = request_body_json.into_bytes();

    // Prepare HTTPS Request
    let request_headers = vec![
        HttpHeader { name: "Authorization".to_string(), value: format!("Bearer {}", api_key) },
        HttpHeader { name: "Content-Type".to_string(), value: "application/json".to_string()},
    ];

    let request = CanisterHttpRequestArgument {
        url: OPENAI_API_URL.to_string(),
        method: HttpMethod::POST,
        body: Some(request_body_bytes),
        max_response_bytes: Some(2048), // Limit response size
        transform: None, // No transform used for simplicity
        headers: request_headers,
    };

    // Make HTTPS Outcall
    ic_cdk::println!("Making HTTPS outcall to OpenAI...");
    match http_request(request, HTTP_REQUEST_CYCLES).await {
        Ok((response,)) => {
            ic_cdk::println!("Received response, status: {}", response.status);
            if response.status >= 200 && response.status < 300 {
                // Parse successful response
                match serde_json::from_slice::<OpenAIResponse>(&response.body) {
                    Ok(openai_response) => {
                        if let Some(choice) = openai_response.choices.first() {
                            ic_cdk::println!("Successfully parsed dare from LLM.");
                            // Clean the response text
                            let dare_text = choice.message.content.trim().trim_matches('"').to_string();
                            if dare_text.is_empty() {
                                Err("LLM returned an empty dare.".to_string())
                            } else {
                                Ok(dare_text)
                            }
                        } else {
                            Err("LLM response contained no choices.".to_string())
                        }
                    }
                    Err(e) => {
                        let raw_body = String::from_utf8_lossy(&response.body);
                        ic_cdk::println!("Failed to parse JSON response: {:?}\nRaw Body: {}", e, raw_body);
                        Err(format!("LLM Response Parse Error: {} (Check raw body in logs)", e))
                    }
                }
            } else {
                // Handle HTTP error status codes
                let raw_body = String::from_utf8_lossy(&response.body);
                ic_cdk::println!("HTTP Error Status: {}, Body: {}", response.status, raw_body);
                Err(format!("LLM API Error (Status {}): {}", response.status, raw_body))
            }
        }
        Err((code, message)) => {
            // Handle canister HTTPS outcall errors
            ic_cdk::println!("HTTPS Outcall failed: {:?} {}", code, message);
            Err(format!("HTTPS Outcall Error: {:?} {}", code, message))
        }
    }
}