use chrono::DateTime;

/// Extracts organization name from full repository name (e.g. "org/repo" -> "org")
pub fn extract_org_name(full_repo_name: &str) -> String {
    if let Some(pos) = full_repo_name.find('/') {
        full_repo_name[..pos].to_string()
    } else {
        full_repo_name.to_string() // Return the full name if no slash is found
    }
}

/// Parses ISO 8601 format date string to Unix timestamp
pub fn parse_iso8601(date_str: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let dt = DateTime::parse_from_rfc3339(date_str)?;
    Ok(dt.timestamp() as u64)
}

/// Parses duration string (e.g. "1h", "30m", "2d") to Duration
pub fn parse_duration(
    duration_str: &str,
) -> Result<std::time::Duration, Box<dyn std::error::Error + Send + Sync>> {
    let duration_str = duration_str.trim();
    if duration_str.is_empty() {
        return Ok(std::time::Duration::from_secs(0));
    }

    if duration_str.len() < 2 {
        return Err("Duration string too short".into());
    }

    // Check for two-character units first
    if duration_str.len() >= 2 {
        let last_two = &duration_str[duration_str.len() - 2..];
        let first_part = &duration_str[..duration_str.len() - 2];

        match last_two {
            "ms" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_millis(num));
                }
            }
            "hr" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60));
                }
            }
            "mo" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24 * 30)); // 月を30日として計算
                }
            }
            "yr" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24 * 365)); // 年を365日として計算
                }
            }
            _ => {
                // Not a two-character unit, continue to check one-character units
            }
        }
    }

    // Check for one-character units
    if !duration_str.is_empty() {
        let last_char = &duration_str[duration_str.len() - 1..];
        let first_part = &duration_str[..duration_str.len() - 1];

        match last_char {
            "s" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num));
                }
            }
            "m" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60));
                }
            }
            "h" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60));
                }
            }
            "d" => {
                if !first_part.is_empty() {
                    let num = first_part.parse::<u64>()?;
                    return Ok(std::time::Duration::from_secs(num * 60 * 60 * 24));
                }
            }
            _ => {
                // Not a recognized unit
            }
        }
    }

    Err("Invalid duration format".into())
}
