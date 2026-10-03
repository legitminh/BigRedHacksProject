use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct FrontmostInfo {
    pub app_name: String,
    pub window_title: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct DistractionHit {
    pub label: &'static str,
    pub detail: String,
    /// True when the distraction is the focused app/tab; false if open in background.
    pub focused: bool,
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

/// Scan frontmost app + ALL open browser tabs for off-task sites (YouTube in background, etc.).
pub fn scan_distractions(front: &FrontmostInfo) -> Option<DistractionHit> {
    if let Some(label) = classify_blob(&front.blob(), &front.app_name) {
        return Some(DistractionHit {
            label,
            detail: front.summary(),
            focused: true,
        });
    }

    #[cfg(target_os = "macos")]
    {
        for url in all_browser_urls() {
            let lower = url.to_lowercase();
            if let Some(label) = classify_url(&lower) {
                // Skip if this is literally the focused URL (already handled).
                if !front.url.is_empty() && urls_similar(&front.url, &url) {
                    continue;
                }
                return Some(DistractionHit {
                    label,
                    detail: url_host(&url).unwrap_or(url),
                    focused: false,
                });
            }
        }
    }

    None
}

/// Compact hint list for Gemini (open off-task tabs / mail / shopping).
pub fn open_context_hints(front: &FrontmostInfo) -> String {
    let mut hints = Vec::new();
    hints.push(format!("frontmost={}", front.summary()));
    #[cfg(target_os = "macos")]
    {
        let mut off_task = Vec::new();
        for url in all_browser_urls().into_iter().take(40) {
            let lower = url.to_lowercase();
            if let Some(label) = classify_url(&lower) {
                off_task.push(format!("{label}:{}", url_host(&url).unwrap_or(url)));
            }
        }
        if !off_task.is_empty() {
            hints.push(format!("open_off_task_tabs={}", off_task.join(" | ")));
        }
    }
    hints.join("\n")
}

pub fn distraction_coach_line(hit: &DistractionHit) -> String {
    let where_ = if hit.focused {
        String::new()
    } else {
        format!(" (still open: {})", hit.detail)
    };
    match hit.label {
        "texting" => format!(
            "Texting pulls you off your lock-in{where_} — finish later and get back to the goal."
        ),
        "email" => format!(
            "Email isn’t your lock-in goal{where_} — close the inbox and return to the work."
        ),
        "shopping" => format!(
            "Shopping tabs aren’t the goal{where_} — close the store and get back to lock-in."
        ),
        "youtube" => {
            if hit.focused {
                "YouTube isn’t the goal — close it and return to your lock-in work.".into()
            } else {
                format!(
                    "YouTube is still open in the background{where_} — close that tab so it doesn’t pull you back."
                )
            }
        }
        "instagram" => format!(
            "Instagram isn’t the goal{where_} — close it and get back to what you locked in on."
        ),
        other => format!("That’s {other}{where_} — close it and get back to your lock-in goal."),
    }
}

fn classify_blob(blob: &str, app_name: &str) -> Option<&'static str> {
    if app_name.to_lowercase().contains("waypoint") {
        return None;
    }
    let app = app_name.to_lowercase();

    if app == "messages"
        || app.contains("imessage")
        || app.contains("whatsapp")
        || app.contains("telegram")
        || app.contains("signal")
        || app.contains("messenger")
        || app == "texts"
    {
        return Some("texting");
    }
    if app == "mail" || app.contains("outlook") || app.contains("spark") || app.contains("airmail")
    {
        return Some("email");
    }

    classify_url(blob).or_else(|| {
        // App-name fallbacks for native clients.
        if app.contains("youtube") {
            Some("youtube")
        } else if app.contains("instagram") {
            Some("instagram")
        } else {
            None
        }
    })
}

fn classify_url(text: &str) -> Option<&'static str> {
    const RULES: &[(&str, &str)] = &[
        // Video / social
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("instagram.com", "instagram"),
        ("tiktok.com", "tiktok"),
        ("twitter.com", "twitter"),
        ("x.com", "twitter"),
        ("facebook.com", "facebook"),
        ("reddit.com", "reddit"),
        ("discord.com", "discord"),
        ("netflix.com", "netflix"),
        ("twitch.tv", "twitch"),
        ("pinterest.com", "pinterest"),
        ("spotify.com", "spotify"),
        ("open.spotify", "spotify"),
        // Messaging web
        ("web.whatsapp.com", "texting"),
        ("messages.google.com", "texting"),
        ("messenger.com", "texting"),
        ("telegram.org", "texting"),
        // Email
        ("mail.google.com", "email"),
        ("outlook.live.com", "email"),
        ("outlook.office.com", "email"),
        ("outlook.office365.com", "email"),
        ("mail.yahoo.com", "email"),
        ("proton.me/mail", "email"),
        ("icloud.com/mail", "email"),
        // Shopping
        ("amazon.", "shopping"),
        ("ebay.", "shopping"),
        ("etsy.com", "shopping"),
        ("walmart.com", "shopping"),
        ("target.com", "shopping"),
        ("bestbuy.com", "shopping"),
        ("shopify", "shopping"),
        ("aliexpress.", "shopping"),
        ("newegg.com", "shopping"),
        ("costco.com", "shopping"),
        ("nike.com", "shopping"),
        ("adidas.com", "shopping"),
        ("apple.com/shop", "shopping"),
        ("store.steampowered", "shopping"),
    ];
    for (needle, label) in RULES {
        if text.contains(needle) {
            return Some(label);
        }
    }
    // Loose tokens for titles/blobs
    if text.contains("youtube") {
        return Some("youtube");
    }
    if text.contains("instagram") {
        return Some("instagram");
    }
    if text.contains("gmail") || text.contains("inbox (" ) {
        return Some("email");
    }
    None
}

#[cfg(target_os = "macos")]
fn all_browser_urls() -> Vec<String> {
    let mut urls = Vec::new();
    for script in [
        CHROME_ALL_TABS,
        SAFARI_ALL_TABS,
        BRAVE_ALL_TABS,
        ARC_ALL_TABS,
        EDGE_ALL_TABS,
    ] {
        if let Ok(raw) = osascript(script) {
            for line in raw.lines() {
                let u = line.trim();
                if u.starts_with("http://") || u.starts_with("https://") {
                    urls.push(u.to_string());
                }
            }
        }
    }
    urls
}

#[cfg(target_os = "macos")]
const CHROME_ALL_TABS: &str = r#"tell application "Google Chrome"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      set out to out & (URL of t as text) & linefeed
    end repeat
  end repeat
  return out
end tell"#;

#[cfg(target_os = "macos")]
const SAFARI_ALL_TABS: &str = r#"tell application "Safari"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      try
        set out to out & (URL of t as text) & linefeed
      end try
    end repeat
  end repeat
  return out
end tell"#;

#[cfg(target_os = "macos")]
const BRAVE_ALL_TABS: &str = r#"tell application "Brave Browser"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      set out to out & (URL of t as text) & linefeed
    end repeat
  end repeat
  return out
end tell"#;

#[cfg(target_os = "macos")]
const ARC_ALL_TABS: &str = r#"tell application "Arc"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      try
        set out to out & (URL of t as text) & linefeed
      end try
    end repeat
  end repeat
  return out
end tell"#;

#[cfg(target_os = "macos")]
const EDGE_ALL_TABS: &str = r#"tell application "Microsoft Edge"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      set out to out & (URL of t as text) & linefeed
    end repeat
  end repeat
  return out
end tell"#;

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

fn urls_similar(a: &str, b: &str) -> bool {
    let ha = url_host(a).unwrap_or_else(|| a.to_lowercase());
    let hb = url_host(b).unwrap_or_else(|| b.to_lowercase());
    ha == hb || a == b
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
