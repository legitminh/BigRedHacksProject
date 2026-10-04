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
        let mut title = osascript(
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
        // Browser tab title is often more reliable than the process window name.
        if let Some(tab_title) = browser_active_title(&app) {
            if !tab_title.is_empty() {
                title = tab_title;
            }
        }
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
/// `goals` lets YouTube study lectures pass; everything else YouTube interrupts.
pub fn evaluate_focus(goals: &str) -> Result<FocusEvent, String> {
    let info = frontmost_info()?;

    // Still scan browser tabs while Waypoint is focused (session UI / overlay).
    let waypoint_focused = info.app_name.to_lowercase().contains("waypoint");

    if !waypoint_focused {
        if let Some(label) = hard_app_label(&info.app_name) {
            return Ok(FocusEvent::Hard(DistractionHit {
                label,
                detail: info.summary(),
                focused: true,
            }));
        }

        // Focused browser URL / window / tab title.
        let focused_label = classify_focus_label(&info);

        if let Some(label) = focused_label {
            return classify_site(label, info, goals, true);
        }
    }

    // Background tabs: URL host only — never title keywords (articles about Instagram ≠ Instagram).
    #[cfg(target_os = "macos")]
    {
        for tab in all_browser_tabs().into_iter().take(50) {
            if !info.url.is_empty() && urls_similar(&info.url, &tab.url) {
                continue;
            }
            let url_lower = tab.url.to_lowercase();
            if let Some(label) = classify_url_host(&url_lower) {
                let mut bg = info.clone();
                // Prefer real browser identity over "Waypoint" when nagging about a tab.
                if waypoint_focused || bg.app_name.is_empty() {
                    bg.app_name = "Browser".into();
                }
                bg.url = tab.url.clone();
                bg.window_title = tab.title.clone();
                let focused = !waypoint_focused
                    && !info.url.is_empty()
                    && urls_similar(&info.url, &tab.url);
                return classify_site(label, bg, goals, focused);
            }
        }
    }

    Ok(FocusEvent::Clear(info))
}

fn classify_focus_label(info: &FrontmostInfo) -> Option<&'static str> {
    // URL host first; title only with brand-chrome patterns (… - Instagram), not loose substrings.
    classify_url_host(&info.url.to_lowercase())
        .or_else(|| classify_title_label(&info.window_title.to_lowercase()))
}

fn classify_title_label(title: &str) -> Option<&'static str> {
    // Require the brand to be the site chrome, e.g. "Reel title - Instagram", not
    // "Why Instagram changed its feed" in a news/docs tab.
    for (brand, label) in [
        ("youtube", "youtube"),
        ("instagram", "instagram"),
        ("tiktok", "tiktok"),
        ("reddit", "reddit"),
        ("netflix", "netflix"),
        ("discord", "discord"),
    ] {
        if title_is_brand_chrome(title, brand) {
            return Some(label);
        }
    }
    if title.starts_with("inbox (") || title.contains(" - gmail") || title.ends_with("gmail") {
        return Some("email");
    }
    None
}

fn title_is_brand_chrome(title: &str, brand: &str) -> bool {
    let t = title.trim();
    if t == brand {
        return true;
    }
    for sep in [" - ", " | ", " • ", " – ", " — "] {
        if let Some((_, right)) = t.rsplit_once(sep) {
            if right.trim() == brand {
                return true;
            }
        }
    }
    false
}

fn classify_site(
    label: &'static str,
    info: FrontmostInfo,
    goals: &str,
    focused: bool,
) -> Result<FocusEvent, String> {
    // YouTube: interrupt by default. Only allow through when title clearly matches study goals.
    if label == "youtube" || label == "video" {
        let page_text = if focused {
            page_context_blob(&info)
        } else {
            format!("{}\n{}", info.window_title, info.url)
        };
        if local_context_guess("youtube", &page_text, goals) == Some(true) {
            return Ok(FocusEvent::NeedsJudgment {
                kind: "youtube",
                info,
                page_text,
            });
        }
        // Unknown / entertainment → hard nag (do not wait on a tiny model that may say on-task).
        return Ok(FocusEvent::Hard(DistractionHit {
            label: "youtube",
            detail: if info.window_title.is_empty() {
                info.summary()
            } else {
                truncate(&info.window_title, 64)
            },
            focused,
        }));
    }

    if is_ambiguous(label) {
        let page_text = page_context_blob(&info);
        return Ok(FocusEvent::NeedsJudgment {
            kind: label,
            info,
            page_text,
        });
    }

    Ok(FocusEvent::Hard(DistractionHit {
        label,
        detail: info.summary(),
        focused,
    }))
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

/// Cheap local guess for YouTube / video titles before the Ollama text judge runs.
/// `Some(true)` = looks study-related, `Some(false)` = entertainment, `None` = unclear.
pub fn local_context_guess(kind: &str, page_text: &str, goals: &str) -> Option<bool> {
    // Only scan the page/title — never let goal words (e.g. "course") mark every video as study.
    let blob = page_text.to_lowercase();
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

/// Host / path evidence only — never bare brand words (avoids "Instagram" in articles).
fn classify_url_host(text: &str) -> Option<&'static str> {
    const RULES: &[(&str, &str)] = &[
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("instagram.com", "instagram"),
        ("tiktok.com", "tiktok"),
        ("twitter.com", "twitter"),
        ("https://x.com/", "twitter"),
        ("http://x.com/", "twitter"),
        ("www.x.com/", "twitter"),
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
        ("aliexpress.", "shopping"),
        ("newegg.com", "shopping"),
        ("costco.com", "shopping"),
        ("apple.com/shop", "shopping"),
        ("store.steampowered", "shopping"),
    ];
    for (needle, label) in RULES {
        if text.contains(needle) {
            return Some(label);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct BrowserTab {
    url: String,
    title: String,
}

/// Title + URL + short body text for cheap local-judge context (no screenshot tokens).
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
    let (apple_script, jxa) = if app.contains("google chrome") || app == "chrome" {
        (
            r#"tell application "Google Chrome"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#,
            Some(r#"Application("Google Chrome").windows[0].activeTab().url()"#),
        )
    } else if app.contains("safari") {
        (
            r#"tell application "Safari"
  if (count of windows) is 0 then return ""
  return URL of current tab of front window
end tell"#,
            Some(r#"Application("Safari").windows[0].currentTab().url()"#),
        )
    } else if app.contains("brave") {
        (
            r#"tell application "Brave Browser"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#,
            None,
        )
    } else if app.contains("arc") {
        (
            r#"tell application "Arc"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#,
            None,
        )
    } else if app.contains("microsoft edge") || app.contains("edge") {
        (
            r#"tell application "Microsoft Edge"
  if (count of windows) is 0 then return ""
  return URL of active tab of front window
end tell"#,
            None,
        )
    } else {
        return None;
    };

    if let Ok(url) = osascript(apple_script) {
        let url = url.trim().to_string();
        if !url.is_empty() && url != "missing value" {
            return Some(url);
        }
    }
    if let Some(jxa) = jxa {
        if let Ok(url) = osascript_jxa(jxa) {
            let url = url.trim().to_string();
            if !url.is_empty() && url != "undefined" && url != "null" {
                return Some(url);
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn browser_active_title(app_name: &str) -> Option<String> {
    let app = app_name.to_lowercase();
    let script = if app.contains("google chrome") || app == "chrome" {
        r#"tell application "Google Chrome"
  if (count of windows) is 0 then return ""
  return title of active tab of front window
end tell"#
    } else if app.contains("safari") {
        r#"tell application "Safari"
  if (count of windows) is 0 then return ""
  return name of current tab of front window
end tell"#
    } else if app.contains("brave") {
        r#"tell application "Brave Browser"
  if (count of windows) is 0 then return ""
  return title of active tab of front window
end tell"#
    } else if app.contains("arc") {
        r#"tell application "Arc"
  if (count of windows) is 0 then return ""
  return title of active tab of front window
end tell"#
    } else if app.contains("microsoft edge") || app.contains("edge") {
        r#"tell application "Microsoft Edge"
  if (count of windows) is 0 then return ""
  return title of active tab of front window
end tell"#
    } else {
        return None;
    };
    let title = osascript(script).ok()?.trim().to_string();
    if title.is_empty() || title == "missing value" {
        None
    } else {
        Some(title)
    }
}

#[cfg(target_os = "macos")]
fn osascript_jxa(source: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", source])
        .output()
        .map_err(|e| format!("osascript jxa failed: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript jxa error: {err}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    match evaluate_focus("") {
        Ok(FocusEvent::Hard(hit)) => Some(hit),
        Ok(FocusEvent::NeedsJudgment { kind, info, .. }) => {
            let _ = (kind, info, front);
            None
        }
        _ => None,
    }
}
