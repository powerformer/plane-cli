use crate::core::app::AppState;
use crate::core::request::Client;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ApiMeOptions {
    pub json: bool,
}

pub fn me(state: &AppState, options: ApiMeOptions) -> Result<String, String> {
    let client = Client::from_state(state).map_err(|error| error.to_string())?;
    let user = client
        .get("users/me/", &[])
        .map_err(|error| error.to_string())?;
    if options.json {
        return serde_json::to_string_pretty(&user)
            .map(|json| format!("{json}\n"))
            .map_err(|error| format!("failed to render API response JSON: {error}"));
    }
    Ok(render_me(client.base_url(), &user))
}

fn render_me(api_base_url: &str, user: &Value) -> String {
    let id = string_field(user, "id").unwrap_or_else(|| "<unknown>".to_string());
    let email = string_field(user, "email").unwrap_or_else(|| "<unknown>".to_string());
    let display_name = string_field(user, "display_name")
        .or_else(|| string_field(user, "displayName"))
        .unwrap_or_else(|| {
            let first = string_field(user, "first_name").unwrap_or_default();
            let last = string_field(user, "last_name").unwrap_or_default();
            let full = format!("{first} {last}").trim().to_string();
            if full.is_empty() {
                "<unknown>".to_string()
            } else {
                full
            }
        });

    format!(
        "Plane API smoke ok\napi_base_url: {api_base_url}\nuser: {display_name} <{email}>\nid: {id}\n"
    )
}

fn string_field(user: &Value, field: &str) -> Option<String> {
    user.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_me_uses_stable_user_fields() {
        let output = render_me(
            "https://plane.example.test/api/v1",
            &json!({
                "id": "user-id",
                "display_name": "Ada Lovelace",
                "email": "ada@example.test"
            }),
        );

        assert!(output.contains("Plane API smoke ok"));
        assert!(output.contains("Ada Lovelace <ada@example.test>"));
        assert!(output.contains("id: user-id"));
    }
}
