use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct FrontmostInfo {
    pub app_name: String,
    pub window_title: String,
    pub url: String,
}

impl FrontmostInfo {
    pub fn blob(&self) -> String {
        format!("{} {} {}", self.app_name, self.window_title, self.url).to_lowercase()
    }

    pub fn summary(&self) -> String {
        if !self.url.is_empty() {
            let host = url_host(&self.url).unwrap_or_else(|| self.url.clone());
            format!("{} · {}", self.app_name, host)
        } else if !self.window_title.is_empty() {
            format!("{} · {}", self.app_name, truncate(&self.window_title, 48))
        } else {
            self.app_name.clone()
        }
    }
}

/// Fast local signal (no Gemini): frontmost macOS app, window title, and browser URL.
pub fn frontmost_info() -> Result<FrontmostInfo, String> {
    #[cfg(target_os = "macos")]
    {
        let app = osascript(
            r#"tell application "System Events" to get name of first application process whose frontmost is true"#,
        )?;
        let app = app.trim().to_string();
        let title = osascript(
            r#"tell application "System Events"
  tell (first application process whose frontmost is true)
    if (count of windows) > 0 then
      return name of window 1
    end if
  end tell
  return ""
end tell"#,
        )
        .unwrap_or_default()
        .trim()
        .to_string();
        let url = browser_active_url(&app).unwrap_or_default();
        Ok(FrontmostInfo {
            app_name: app,
            window_title: title,
            url,
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Frontmost-app detection is only wired for macOS right now.".into())
    }
}

/// Returns a short distraction label if the frontmost context looks like social / entertainment / texting.
pub fn social_label(info: &FrontmostInfo) -> Option<&'static str> {
    if info.app_name.to_lowercase().contains("waypoint") {
        return None;
    }

    let app = info.app_name.to_lowercase();
    let blob = info.blob();

    // Messaging apps (texting) — match on app name first so Messages is always caught.
    if app == "messages"
        || app.contains("imessage")
        || app.contains("whatsapp")
        || app.contains("telegram")
        || app.contains("signal")
        || app.contains("messenger")
        || app.contains("slack")
        || app == "texts"
        || app.contains("android messages")
    {
        return Some("texting");
    }

    const SITES: &[(&str, &str)] = &[
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("youtube", "youtube"),
        ("instagram.com", "instagram"),
        ("instagram", "instagram"),
        ("tiktok.com", "tiktok"),
        ("tiktok", "tiktok"),
        ("twitter.com", "twitter"),
        ("x.com", "twitter"),
        ("facebook.com", "facebook"),
        ("facebook", "facebook"),
        ("messenger.com", "texting"),
        ("messages.google.com", "texting"),
        ("web.whatsapp.com", "texting"),
        ("reddit.com", "reddit"),
        ("reddit", "reddit"),
        ("discord.com", "discord"),
        ("discord", "discord"),
        ("netflix.com", "netflix"),
        ("netflix", "netflix"),
        ("twitch.tv", "twitch"),
        ("twitch", "twitch"),
        ("pinterest.com", "pinterest"),
        ("spotify.com", "spotify"),
        ("open.spotify", "spotify"),
        ("spotify", "spotify"),
        ("whatsapp", "texting"),
        ("telegram", "texting"),
        ("snapchat", "snapchat"),
        ("imessage", "texting"),
        ("messages", "texting"),
    ];
    for (needle, label) in SITES {
        if blob.contains(needle) {
            return Some(label);
        }
    }
    None
}

pub fn distraction_coach_line(label: &str) -> String {
    match label {
        "texting" => {
            "Texting pulls you off your lock-in — finish the message later and get back to the goal."
                .into()
        }
        "youtube" => "YouTube isn’t the goal — close the tab and return to your lock-in work.".into(),
        "instagram" => {
            "Instagram isn’t the goal — close it and get back to what you locked in on.".into()
        }
        "messages" => {
            "Messages can wait — park the chat and return to your lock-in goal.".into()
        }
        other => format!("That’s {other} — close it and get back to your lock-in goal."),
    }
}

#[cfg(target_os = "macos")]
fn browser_active_url(app_name: &str) -> Option<String> {
    let app = app_name.to_lowercase();
    let script = if app.contains("google chrome") || app == "chrome" {
        r#"tell application "Google Chrome"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#
    } else if app.contains("safari") {
        r#"tell application "Safari"
  if (count of windows) is 0 then return ""
  return URL of current tab of front window
end tell"#
    } else if app.contains("brave") {
        r#"tell application "Brave Browser"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#
    } else if app.contains("arc") {
        r#"tell application "Arc"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#
    } else if app.contains("microsoft edge") || app.contains("edge") {
        r#"tell application "Microsoft Edge"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#
    } else if app.contains("chromium") {
        r#"tell application "Chromium"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#
    } else {
        return None;
    };

    let url = osascript(script).ok()?.trim().to_string();
    if url.is_empty() || url == "missing value" {
        None
    } else {
        Some(url)
    }
}

fn url_host(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme
        .split('/')
        .next()?
        .split('?')
        .next()?
        .to_string();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

#[cfg(target_os = "macos")]
fn osascript(source: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .args(["-e", source])
        .output()
        .map_err(|e| format!("osascript failed: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript error: {err}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
