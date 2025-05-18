use reqwest::header;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenrouterRequestBody {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct ModelResponse {
    short_link: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct OpenrouterResponse {
    #[serde(rename = "id")]
    _id: String,
    choices: Vec<Choice>,
}

pub async fn generate(full_link: &str, token: &str, bad_attempts: &[String]) -> Option<String> {
    let bad_attempts = serde_json::to_string(bad_attempts).unwrap_or("[]".to_string());
    let prompt = format!(
        r#"
Can you suggest a short path for a URL shortener for this URL: '{}'? 
Give only one suggestion. It should be one word, possibly with underscores.
Output have to be in json format, don't write anything except the json.
The following values are prohibited: {} 
Example output:
{}
"#,
        full_link, bad_attempts, "{\"short_link\": \"url\"}"
    );
    let body = OpenrouterRequestBody {
        model: "meta-llama/llama-4-maverick:free".to_string(),
        messages: vec![Message {
            role: "assistant".to_string(),
            content: prompt,
        }],
    };
    let client = reqwest::Client::new();

    let response = match client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Error in openrouter api: {e}");
            return None;
        }
    };

    let openrouter_resp: OpenrouterResponse = match response.json::<OpenrouterResponse>().await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Error in demarshalling api: {e}");
            return None;
        }
    };
    let model_response: ModelResponse =
        serde_json::from_str(&openrouter_resp.choices[0].message.content).ok()?;

    Some(model_response.short_link)
}
