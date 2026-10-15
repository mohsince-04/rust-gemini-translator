use std::process::Command;
use std::fs;
use std::path::Path;

pub fn get_browser_cookies() -> Result<(String, String), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set")?;
    
    let exact_path = format!("{}/.mozilla/firefox/hardened/cookies.sqlite", home);
    let db_path = if Path::new(&exact_path).exists() {
        exact_path
    } else {
        let firefox_dir = format!("{}/.mozilla/firefox", home);
        let output = Command::new("find")
            .arg(&firefox_dir)
            .args(["-maxdepth", "3", "-name", "cookies.sqlite"])
            .output()?;
            
        let stdout_str = String::from_utf8(output.stdout)?;
        let mut found = String::new();
        for line in stdout_str.lines() {
            let trimmed = line.trim();
            if trimmed.contains("hardened") && trimmed.ends_with("cookies.sqlite") {
                found = trimmed.to_string();
                break;
            }
        }
        found
    };

    if db_path.is_empty() {
        return Err("Could not locate cookies.sqlite in the 'hardened' Firefox profile.".into());
    }

    fs::copy(&db_path, "/tmp/cookies_temp.sqlite")?;

    let res = Command::new("sqlite3")
        .arg("/tmp/cookies_temp.sqlite")
        .arg("SELECT name, value FROM moz_cookies WHERE name LIKE '%1PSID%';")
        .output()?;
        
    let result_str = String::from_utf8(res.stdout)?;

    let mut psid = String::new();
    let mut psidts = String::new();

    for line in result_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            if parts[0].trim() == "__Secure-1PSID" {
                psid = parts[1].trim().to_string();
            } else if parts[0].trim() == "__Secure-1PSIDTS" {
                psidts = parts[1].trim().to_string();
            }
        }
    }

    if psid.is_empty() || psidts.is_empty() {
        return Err(format!(
            "Loaded database from [{}] but session cookies were not found. Ensure you are signed into gemini.google.com in your 'hardened' profile.",
            db_path
        ).into());
    }

    Ok((psid, psidts))
}
