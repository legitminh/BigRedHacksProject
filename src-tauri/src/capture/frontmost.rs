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

/// What the local watcher saw this tick.
#[derive(Debug, Clone)]
pub enum FocusEvent {
    /// Definitely off-task — nag immediately (Discord, Instagram, shopping, …).
    Hard(DistractionHit),
    /// Could be study or distraction — judge from page text/title (YouTube, etc.).
    NeedsJudgment {
        kind: &'static str,
        info: FrontmostInfo,
        /// Title + URL + short page excerpt (cheap vs screenshot).
        page_text: String,
    },
    Clear(FrontmostInfo),
}

impl FrontmostInfo {
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

    /// Fingerprint for “app/tab just opened or switched” events.
    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}",
            self.app_name.to_lowercase(),
            self.url.to_lowercase(),
            self.window_title.to_lowercase()
        )
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

/// Classify the current focus for the coach (hard vs needs context vs clear).
pub fn evaluate_focus() -> Result<FocusEvent, String> {
    let info = frontmost_info()?;

    if info.app_name.to_lowercase().contains("waypoint") {
        return Ok(FocusEvent::Clear(info));
    }

    if let Some(label) = hard_app_label(&info.app_name) {
        return Ok(FocusEvent::Hard(DistractionHit {
            label,
            detail: info.summary(),
            focused: true,
        }));
    }

    // Focused browser URL / title.
    let focused_label = classify_url(&info.url.to_lowercase())
        .or_else(|| classify_url(&info.window_title.to_lowercase()));

    if let Some(label) = focused_label {
        if is_ambiguous(label) {
            let page_text = page_context_blob(&info);
            return Ok(FocusEvent::NeedsJudgment {
                kind: label,
                info,
                page_text,
            });
        }
        return Ok(FocusEvent::Hard(DistractionHit {
            label,
            detail: info.summary(),
            focused: true,
        }));
    }

    // Background tabs: hard sites nag; ambiguous (YouTube) also get judgment.
    #[cfg(target_os = "macos")]
    {
        for tab in all_browser_tabs().into_iter().take(40) {
            if !info.url.is_empty() && urls_similar(&info.url, &tab.url) {
                continue;
            }
            let lower = tab.url.to_lowercase();
            if let Some(label) = classify_url(&lower) {
                if is_ambiguous(label) {
                    let mut page_text = format!("{}\n{}", tab.title, tab.url);
                    if page_text.trim().len() < 8 {
                        page_text = tab.url.clone();
                    }
                    let mut bg = info.clone();
                    bg.url = tab.url;
                    bg.window_title = tab.title;
                    return Ok(FocusEvent::NeedsJudgment {
                        kind: label,
                        info: bg,
                        page_text,
                    });
                }
                return Ok(FocusEvent::Hard(DistractionHit {
                    label,
                    detail: url_host(&tab.url).unwrap_or(tab.url),
                    focused: false,
                }));
            }
        }
    }

    Ok(FocusEvent::Clear(info))
}

/// Compact hint list for Gemini vision.
pub fn open_context_hints(front: &FrontmostInfo) -> String {
    let mut hints = Vec::new();
    hints.push(format!("frontmost={}", front.summary()));
    if !front.window_title.is_empty() {
        hints.push(format!("window_title={}", truncate(&front.window_title, 80)));
    }
    #[cfg(target_os = "macos")]
    {
        let mut off_task = Vec::new();
        for tab in all_browser_tabs().into_iter().take(40) {
            let lower = tab.url.to_lowercase();
            if let Some(label) = classify_url(&lower) {
                let host = url_host(&tab.url).unwrap_or(tab.url);
                let title = truncate(&tab.title, 40);
                off_task.push(format!("{label}:{host} ({title})"));
            }
        }
        if !off_task.is_empty() {
            hints.push(format!("open_flagged_tabs={}", off_task.join(" | ")));
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
        "discord" => format!(
            "Discord just pulled you off lock-in{where_} — leave the chat and return to your goal."
        ),
        "youtube" => {
            if hit.focused {
                "This YouTube isn’t helping your lock-in — switch back to the work.".into()
            } else {
                format!(
                    "YouTube is still open in the background{where_} — close entertainment tabs so they don’t pull you back."
                )
            }
        }
        "instagram" => format!(
            "Instagram isn’t the goal{where_} — close it and get back to what you locked in on."
        ),
        other => format!("That’s {other}{where_} — close it and get back to your lock-in goal."),
    }
}

/// Cheap local guess for YouTube / video titles when Gemini text judge is slow.
/// `Some(true)` = looks study-related, `Some(false)` = entertainment, `None` = unclear.
pub fn local_context_guess(kind: &str, page_text: &str, goals: &str) -> Option<bool> {
    let blob = format!("{page_text} {goals}").to_lowercase();
    if kind != "youtube" && kind != "video" {
        return None;
    }

    const STUDY: &[&str] = &[
        "lecture",
        "tutorial",
        "course",
        "exam",
        "homework",
        "explained",
        "walkthrough",
        "crash course",
        "khan academy",
        "mit ocw",
        "calculus",
        "algebra",
        "physics",
        "chemistry",
        "biology",
        "organic",
        "linear algebra",
        "data structure",
        "algorithm",
        "leetcode",
        "coding",
        "programming",
        "how to solve",
        "problem set",
        "review session",
        "study with me",
        "chapter",
        "textbook",
        "proof",
        "theorem",
        "derivation",
        "lab ",
        "workshop",
        "seminar",
        "mooc",
        "coursera",
        "edx",
    ];
    const ENTERTAIN: &[&str] = &[
        "music video",
        "official video",
        "official audio",
        "lyrics",
        "funny",
        "compilation",
        "vlog",
        "prank",
        "reaction",
        "minecraft",
        "fortnite",
        "gameplay",
        "highlights",
        "podcast clip",
        "trailer",
        "asmr",
        "meme",
        "roast",
        "drama",
        "tiktok",
        "shorts",
        "rap ",
        "song ",
        "mv ",
        "live performance",
        "stand-up",
        "stand up",
    ];

    let study_hit = STUDY.iter().any(|k| blob.contains(k));
    let fun_hit = ENTERTAIN.iter().any(|k| blob.contains(k));

    // Goal keywords appearing in the video title/page → strong on-task signal.
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| page_text.to_lowercase().contains(k))
        .count();

    if goal_overlap >= 1 && !fun_hit {
        return Some(true);
    }
    if study_hit && !fun_hit {
        return Some(true);
    }
    if fun_hit && !study_hit {
        return Some(false);
    }
    None
}

fn goal_keywords(goals: &str) -> Vec<String> {
    goals
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() >= 4)
        .filter(|s| {
            !matches!(
                s.as_str(),
                "with" | "this" | "that" | "from" | "study" | "work" | "lock" | "into" | "about"
                    | "have" | "will" | "just" | "want" | "need" | "goal" | "goals" | "session"
            )
        })
        .take(12)
        .collect()
}

fn is_ambiguous(label: &str) -> bool {
    matches!(label, "youtube" | "video")
}

fn hard_app_label(app_name: &str) -> Option<&'static str> {
    let app = app_name.to_lowercase();
    if app.contains("discord") {
        return Some("discord");
    }
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
    if app.contains("instagram") {
        return Some("instagram");
    }
    if app.contains("tiktok") {
        return Some("tiktok");
    }
    if app.contains("netflix") {
        return Some("netflix");
    }
    None
}

fn classify_url(text: &str) -> Option<&'static str> {
    const RULES: &[(&str, &str)] = &[
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("instagram.com", "instagram"),
        ("tiktok.com", "tiktok"),
        ("twitter.com", "twitter"),
        ("x.com", "twitter"),
        ("facebook.com", "facebook"),
        ("reddit.com", "reddit"),
        ("discord.com", "discord"),
        ("discordapp.com", "discord"),
        ("netflix.com", "netflix"),
        ("twitch.tv", "twitch"),
        ("pinterest.com", "pinterest"),
        ("spotify.com", "spotify"),
        ("open.spotify", "spotify"),
        ("web.whatsapp.com", "texting"),
        ("messages.google.com", "texting"),
        ("messenger.com", "texting"),
        ("telegram.org", "texting"),
        ("mail.google.com", "email"),
        ("outlook.live.com", "email"),
        ("outlook.office.com", "email"),
        ("outlook.office365.com", "email"),
        ("mail.yahoo.com", "email"),
        ("proton.me/mail", "email"),
        ("icloud.com/mail", "email"),
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
    if text.contains("instagram") {
        return Some("instagram");
    }
    if text.contains("gmail") || text.contains("inbox (") {
        return Some("email");
    }
    // Avoid classifying every title that mentions "youtube" elsewhere; URL rules cover it.
    None
}

#[derive(Debug, Clone)]
struct BrowserTab {
    url: String,
    title: String,
}

/// Title + URL + short body text for cheap Gemini context (no screenshot tokens).
fn page_context_blob(info: &FrontmostInfo) -> String {
    let mut parts = Vec::new();
    if !info.window_title.is_empty() {
        parts.push(info.window_title.clone());
    }
    if !info.url.is_empty() {
        parts.push(info.url.clone());
    }
    if let Some(excerpt) = grab_page_excerpt(&info.app_name) {
        let cleaned = collapse_ws(&excerpt);
        if !cleaned.is_empty() {
            parts.push(truncate(&cleaned, 1600));
        }
    }
    if parts.is_empty() {
        info.summary()
    } else {
        parts.join("\n")
    }
}

#[cfg(target_os = "macos")]
fn grab_page_excerpt(app_name: &str) -> Option<String> {
    let app = app_name.to_lowercase();
    let script = if app.contains("google chrome") || app == "chrome" {
        r#"tell application "Google Chrome"
  if (count of windows) is 0 then return ""
  tell active tab of front window
    try
      return execute javascript "(() => { const t=(document.title||''); const b=((document.body&&document.body.innerText)||'').replace(/\\s+/g,' ').trim().slice(0,1600); return t+'\\n'+b; })()"
    on error
      return ""
    end try
  end tell
end tell"#
    } else if app.contains("brave") {
        r#"tell application "Brave Browser"
  if (count of windows) is 0 then return ""
  tell active tab of front window
    try
      return execute javascript "(() => { const t=(document.title||''); const b=((document.body&&document.body.innerText)||'').replace(/\\s+/g,' ').trim().slice(0,1600); return t+'\\n'+b; })()"
    on error
      return ""
    end try
  end tell
end tell"#
    } else if app.contains("safari") {
        r#"tell application "Safari"
  if (count of windows) is 0 then return ""
  try
    return do JavaScript "(() => { const t=(document.title||''); const b=((document.body&&document.body.innerText)||'').replace(/\\s+/g,' ').trim().slice(0,1600); return t+'\\n'+b; })()" in current tab of front window
  on error
    return ""
  end try
end tell"#
    } else if app.contains("arc") {
        // Arc often blocks JS automation — title/URL still help.
        return None;
    } else if app.contains("microsoft edge") || app.contains("edge") {
        r#"tell application "Microsoft Edge"
  if (count of windows) is 0 then return ""
  tell active tab of front window
    try
      return execute javascript "(() => { const t=(document.title||''); const b=((document.body&&document.body.innerText)||'').replace(/\\s+/g,' ').trim().slice(0,1600); return t+'\\n'+b; })()"
    on error
      return ""
    end try
  end tell
end tell"#
    } else {
        return None;
    };

    let raw = osascript(script).ok()?.trim().to_string();
    if raw.is_empty() || raw == "missing value" {
        None
    } else {
        Some(raw)
    }
}

#[cfg(not(target_os = "macos"))]
fn grab_page_excerpt(_app_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn all_browser_tabs() -> Vec<BrowserTab> {
    let mut tabs = Vec::new();
    for script in [
        CHROME_ALL_TABS,
        SAFARI_ALL_TABS,
        BRAVE_ALL_TABS,
        ARC_ALL_TABS,
        EDGE_ALL_TABS,
    ] {
        if let Ok(raw) = osascript(script) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // url|||title
                if let Some((url, title)) = line.split_once("|||") {
                    let url = url.trim();
                    if url.starts_with("http://") || url.starts_with("https://") {
                        tabs.push(BrowserTab {
                            url: url.to_string(),
                            title: title.trim().to_string(),
                        });
                    }
                } else if line.starts_with("http://") || line.starts_with("https://") {
                    tabs.push(BrowserTab {
                        url: line.to_string(),
                        title: String::new(),
                    });
                }
            }
        }
    }
    tabs
}

#[cfg(target_os = "macos")]
const CHROME_ALL_TABS: &str = r#"tell application "Google Chrome"
  if not running then return ""
  set out to ""
  repeat with w in windows
    repeat with t in tabs of w
      set out to out & (URL of t as text) & "|||" & (title of t as text) & linefeed
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
        set out to out & (URL of t as text) & "|||" & (name of t as text) & linefeed
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
      set out to out & (URL of t as text) & "|||" & (title of t as text) & linefeed
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
        set out to out & (URL of t as text) & "|||" & (title of t as text) & linefeed
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
      set out to out & (URL of t as text) & "|||" & (title of t as text) & linefeed
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

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// Kept for any leftover callers.
#[allow(dead_code)]
pub fn scan_distractions(front: &FrontmostInfo) -> Option<DistractionHit> {
    match evaluate_focus() {
        Ok(FocusEvent::Hard(hit)) => Some(hit),
        Ok(FocusEvent::NeedsJudgment { kind, info, .. }) => {
            // Ambiguous — don't hard-flag from the legacy helper.
            let _ = (kind, info, front);
            None
        }
        _ => None,
    }
}
