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
    /// Definitely off-task — nag immediately (Instagram-class, shopping, texting, …).
    Hard(DistractionHit),
    /// Could be study or distraction — judge from page text/title (YouTube, Discord, …).
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

/// Waypoint itself — never nag about the coach UI.
pub fn is_coach_app(app_name: &str) -> bool {
    app_name.to_lowercase().contains("waypoint")
}

/// Code editors / IDEs / terminals. Not auto on-task — judged against session goals.
pub fn is_editor_app(app_name: &str) -> bool {
    let app = app_name.to_lowercase();
    app.contains("cursor")
        || app == "code"
        || app.contains("visual studio code")
        || app.contains("vscode")
        || app.contains("xcode")
        || app.contains("intellij")
        || app.contains("webstorm")
        || app.contains("pycharm")
        || app.contains("sublime")
        || app.contains("terminal")
        || app.contains("iterm")
        || app.contains("warp")
}

/// Writing / notes apps — usually fine for essay/discussion goals.
pub fn is_writing_app(app_name: &str) -> bool {
    let app = app_name.to_lowercase();
    app.contains("notion")
        || app.contains("obsidian")
        || app.contains("word")
        || app.contains("pages")
        || app.contains("google docs")
        || app.contains("excel")
        || app.contains("numbers")
        || app.contains("powerpoint")
        || app.contains("keynote")
}

/// Legacy helper: coach UI + writing apps (not IDEs — those are goal-checked).
/// Coach OCR skip must use [`is_coach_app`] only — editors are goal-checked, not auto-clear.
#[allow(dead_code)]
pub fn is_productive_work_app(app_name: &str) -> bool {
    is_coach_app(app_name) || is_writing_app(app_name)
}

/// True when the focused window's URL/title corroborates an OCR/distraction label.
/// Canvas (or any non-YouTube host) must never accept a ghost `youtube` OCR label.
///
/// - Known distraction host on the focused URL → must match `label`.
/// - Non-empty focused URL that is not that host (e.g. canvas.cornell.edu) → reject.
/// - Empty URL (AppleScript failed): allow title chrome, else allow OCR (URL-unavailable path).
pub fn focus_supports_distraction_label(label: &str, info: &FrontmostInfo) -> bool {
    let url_l = info.url.to_lowercase();
    let title_l = info.window_title.to_lowercase();
    let want = match label {
        "video" => "youtube",
        other => other,
    };

    if let Some(from_url) = classify_url_host(&url_l) {
        return from_url == want;
    }
    if !url_l.trim().is_empty() {
        // Focused on a real host that isn't a known distraction for this label
        // (Canvas, docs, GitHub, …) — never trust OCR brand ghosts.
        return false;
    }
    if let Some(from_title) = classify_title_label(&title_l) {
        return from_title == want;
    }
    // No URL and no brand-chrome title — OCR may be the only signal (automation gap).
    true
}

pub fn is_browser_app(app_name: &str) -> bool {
    let app = app_name.to_lowercase();
    app.contains("safari")
        || app.contains("chrome")
        || app.contains("firefox")
        || app.contains("edge")
        || app.contains("brave")
        || app.contains("arc")
        || app.contains("opera")
}

/// Classify the current focus for the coach (hard vs needs context vs clear).
/// Verdict comes from the **focused** window's URL/title/page text + goals.
/// Background YouTube must not override an on-topic Canvas tab.
pub fn evaluate_focus(goals: &str) -> Result<FocusEvent, String> {
    let info = frontmost_info()?;

    // Focused URL / browser / known apps decide from the frontmost window alone —
    // never pull Contextual labels (YouTube) from background tabs.
    if is_coach_app(&info.app_name)
        || hard_app_label(&info.app_name).is_some()
        || contextual_app_label(&info.app_name).is_some()
        || is_editor_app(&info.app_name)
        || is_writing_app(&info.app_name)
        || classify_focus_label(&info).is_some()
        || !info.url.trim().is_empty()
        || is_browser_app(&info.app_name)
    {
        return decide_focus(info, goals, &[]);
    }

    // No focused URL: only surface AlwaysOff background tabs (Instagram/shopping).
    #[cfg(target_os = "macos")]
    {
        let tabs = all_browser_tabs();
        return decide_focus(info, goals, &tabs);
    }

    #[cfg(not(target_os = "macos"))]
    {
        decide_focus(info, goals, &[])
    }
}

/// Pure focus decision (no AppleScript). Used by `evaluate_focus` and unit tests.
fn decide_focus(
    info: FrontmostInfo,
    goals: &str,
    background_tabs: &[BrowserTab],
) -> Result<FocusEvent, String> {
    // While Waypoint itself is frontmost, don't nag about background browser tabs.
    if is_coach_app(&info.app_name) {
        return Ok(FocusEvent::Clear(info));
    }

    if let Some(label) = hard_app_label(&info.app_name) {
        return Ok(FocusEvent::Hard(DistractionHit {
            label,
            detail: info.summary(),
            focused: true,
        }));
    }

    // Discord / Steam / PDF apps: title/context, not app-name alone.
    if let Some(label) = contextual_app_label(&info.app_name) {
        return classify_site(label, info, goals, true);
    }

    // Cursor / IDEs: on-task only when goals look like coding (or title matches goals).
    if is_editor_app(&info.app_name) {
        return classify_editor(info, goals);
    }

    // Docs / notes: clear for writing/coursework goals; otherwise judge.
    if is_writing_app(&info.app_name) {
        return classify_writing_app(info, goals);
    }

    // Focused browser URL / window / tab title.
    let focused_label = classify_focus_label(&info);

    if let Some(label) = focused_label {
        return classify_site(label, info, goals, true);
    }

    // Focused browser (or any app with a URL) and no distraction host → judge THIS
    // screen against goals. Do NOT scan background tabs for YouTube and claim the
    // student is on YouTube while Canvas is frontmost.
    if !info.url.trim().is_empty() || is_browser_app(&info.app_name) {
        return classify_site("screen", info, goals, true);
    }

    // No focused URL: only surface AlwaysOff background tabs (Instagram/shopping).
    // Never hard-nag Contextual surfaces (YouTube) from a background tab alone —
    // that produced "That's youtube" while the student was on Canvas.
    for tab in background_tabs.iter().take(50) {
        let url_lower = tab.url.to_lowercase();
        if let Some(label) = classify_url_host(&url_lower) {
            if surface_policy(label) != SurfacePolicy::AlwaysOff {
                continue;
            }
            let mut bg = info.clone();
            if bg.app_name.is_empty() {
                bg.app_name = "Browser".into();
            }
            bg.url = tab.url.clone();
            bg.window_title = tab.title.clone();
            return classify_site(label, bg, goals, false);
        }
    }

    Ok(FocusEvent::Clear(info))
}

fn classify_editor(info: FrontmostInfo, goals: &str) -> Result<FocusEvent, String> {
    let page_text = format!("{}\n{}", info.window_title, info.app_name);
    match local_context_guess("editor", &page_text, goals) {
        Some(true) => Ok(FocusEvent::Clear(info)),
        Some(false) => Ok(FocusEvent::Hard(DistractionHit {
            label: "editor",
            detail: info.summary(),
            focused: true,
        })),
        None => Ok(FocusEvent::NeedsJudgment {
            kind: "editor",
            info,
            page_text,
        }),
    }
}

fn classify_writing_app(info: FrontmostInfo, goals: &str) -> Result<FocusEvent, String> {
    let page_text = format!("{}\n{}", info.window_title, info.app_name);
    // Essays / discussion posts / notes — writing apps are usually the work.
    if goals_look_like_writing(goals) || goals_look_like_coursework(goals) {
        return Ok(FocusEvent::Clear(info));
    }
    if goals_look_like_coding(goals) {
        // Coding mission in Pages/Word is odd — soft-judge.
        return Ok(FocusEvent::NeedsJudgment {
            kind: "screen",
            info,
            page_text,
        });
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

/// Host/app distraction policy. Extend ALWAYS_OFF / CONTEXTUAL lists — don't special-case
/// brand names inside the coach loop.
///
/// Product principle: on-task vs off-task comes from **session goals + screen context**,
/// not whole-category bans. YouTube, papers/PDFs, Discord, and gaming are Contextual;
/// Instagram-class social / shopping / texting stay AlwaysOff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfacePolicy {
    /// Never study (Instagram-class social, shopping, pure texting, streaming apps, …).
    AlwaysOff,
    /// Can be study or distraction — decide from title/URL/page text + goals.
    Contextual,
}

fn surface_policy(label: &str) -> SurfacePolicy {
    match label {
        // Contextual: judge from goals + page/window context, not the category alone.
        // "screen" / "editor" are focused work surfaces — never AlwaysOff defaults.
        "youtube" | "video" | "discord" | "reading" | "gaming" | "screen" | "editor" => {
            SurfacePolicy::Contextual
        }
        // Always-off categories (add hosts/apps via classify_url_host / hard_app_label).
        "instagram" | "tiktok" | "shopping" | "texting" | "email" | "netflix" | "twitch"
        | "spotify" | "facebook" | "twitter" | "reddit" | "pinterest" | "snapchat" => {
            SurfacePolicy::AlwaysOff
        }
        // Unknown distraction labels stay hard-off (safe default).
        _ => SurfacePolicy::AlwaysOff,
    }
}

fn classify_site(
    label: &'static str,
    info: FrontmostInfo,
    goals: &str,
    focused: bool,
) -> Result<FocusEvent, String> {
    let page_text = if focused {
        page_context_blob(&info)
    } else {
        format!("{}\n{}", info.window_title, info.url)
    };

    if surface_policy(label) == SurfacePolicy::Contextual {
        // Cheap title/URL guess first; entertainment → hard nag without waiting on LLM.
        match local_context_guess(label, &page_text, goals) {
            Some(false) => {
                return Ok(FocusEvent::Hard(DistractionHit {
                    label,
                    detail: if info.window_title.is_empty() {
                        info.summary()
                    } else {
                        truncate(&info.window_title, 64)
                    },
                    focused,
                }));
            }
            Some(true) => {
                // Focused LMS / generic screen with clear goal overlap → Clear immediately
                // so OCR/background tabs cannot re-label the student as "youtube".
                // YouTube/video study still goes to NeedsJudgment for local text confirm.
                if matches!(label, "screen" | "editor") {
                    return Ok(FocusEvent::Clear(info));
                }
                return Ok(FocusEvent::NeedsJudgment {
                    kind: label,
                    info,
                    page_text,
                });
            }
            None => {
                // YouTube/video/gaming: interrupt by default when context is unclear
                // (tiny models often mark entertainment on-task). screen / editor /
                // Discord / reading stay judgment-gated (never Hard-as-youtube).
                if matches!(label, "youtube" | "video" | "gaming") {
                    return Ok(FocusEvent::Hard(DistractionHit {
                        label,
                        detail: if info.window_title.is_empty() {
                            info.summary()
                        } else {
                            truncate(&info.window_title, 64)
                        },
                        focused,
                    }));
                }
                return Ok(FocusEvent::NeedsJudgment {
                    kind: label,
                    info,
                    page_text,
                });
            }
        }
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
        "reading" => format!(
            "That reading isn’t tied to your lock-in goals{where_} — switch back to the work that matches this session."
        ),
        "gaming" => format!(
            "This game isn’t part of your lock-in goals{where_} — leave it and return to the work."
        ),
        other => format!("That’s {other}{where_} — close it and get back to your lock-in goal."),
    }
}

/// Cheap local guess for contextual surfaces before the Ollama text judge.
/// `Some(true)` = looks on-task for goals, `Some(false)` = off-task, `None` = unclear.
/// Ignores tab-group chrome like "School" — only title/URL/page_text + goal overlap count.
pub fn local_context_guess(kind: &str, page_text: &str, goals: &str) -> Option<bool> {
    // Only scan the page/title — never let goal words (e.g. "course") mark every video as study.
    let blob = page_text.to_lowercase();
    // Tab-group labels ("School") are not evidence of being on-task.
    let blob = blob
        .replace("tab group", " ")
        .replace("school group", " ");

    match kind {
        "youtube" | "video" => guess_youtube_context(&blob, page_text, goals),
        "discord" => guess_discord_context(&blob, goals),
        "reading" => guess_reading_context(&blob, page_text, goals),
        "gaming" => guess_gaming_context(page_text, goals),
        "screen" => guess_screen_context(&blob, page_text, goals),
        "editor" => guess_editor_context(page_text, goals),
        _ => None,
    }
}

/// Focused non-distraction browser tab (Canvas, docs, etc.).
fn guess_screen_context(blob: &str, page_text: &str, goals: &str) -> Option<bool> {
    let head = title_url_head(page_text);
    let hay = format!("{head}\n{blob}");
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| head.contains(k) || blob.contains(k.as_str()))
        .count();

    // LMS / course sites matching session goals → on-task (never youtube).
    if is_study_lms_host(&hay) {
        if goal_overlap >= 1
            || goals_look_like_coursework(goals)
            || goals_look_like_writing(goals)
        {
            return Some(true);
        }
        // Canvas with unrelated goals still needs judgment — not Hard.
        return None;
    }

    if goal_overlap >= 1 {
        return Some(true);
    }
    None
}

/// IDE / terminal: on-task only for coding goals or when the window title matches goals.
fn guess_editor_context(page_text: &str, goals: &str) -> Option<bool> {
    let head = title_url_head(page_text);
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| head.contains(k))
        .count();

    if goal_overlap >= 1 {
        return Some(true);
    }
    if goals_look_like_coding(goals) {
        return Some(true);
    }
    // Essay / discussion / coursework missions → Cursor is off-task.
    if goals_look_like_writing(goals) || goals_look_like_coursework(goals) {
        return Some(false);
    }
    None
}

/// Canvas / LMS / course portals — study hosts (goal-checked, not auto-youtube).
pub fn is_study_lms_host(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("canvas.")
        || t.contains("instructure.com")
        || t.contains("blackboard.")
        || t.contains("blackboard.com")
        || t.contains("brightspace.")
        || t.contains("schoology.com")
        || t.contains("moodle.")
}

pub fn goals_look_like_coding(goals: &str) -> bool {
    let g = goals.to_lowercase();
    const KEYS: &[&str] = &[
        "code",
        "coding",
        "program",
        "programming",
        "debug",
        "leetcode",
        "software",
        "hackathon",
        "rust",
        "python",
        "javascript",
        "typescript",
        "refactor",
        "github",
        "compiler",
        "algorithm",
        "data structure",
        "frontend",
        "backend",
        "ide",
        "c++",
        "java",
    ];
    KEYS.iter().any(|k| g.contains(k))
}

pub fn goals_look_like_writing(goals: &str) -> bool {
    let g = goals.to_lowercase();
    const KEYS: &[&str] = &[
        "essay",
        "write",
        "writing",
        "draft",
        "discussion",
        "paper",
        "thesis",
        "paragraph",
        "journal",
        "blog",
        "story",
        "narrative",
        "reflection",
        "response paper",
        "discussion post",
    ];
    KEYS.iter().any(|k| g.contains(k))
}

pub fn goals_look_like_coursework(goals: &str) -> bool {
    let g = goals.to_lowercase();
    const KEYS: &[&str] = &[
        "homework",
        "assignment",
        "quiz",
        "exam",
        "midterm",
        "problem set",
        "pset",
        "lab report",
        "coursework",
        "schoolwork",
        "canvas",
        "lecture",
        "discussion",
        "finish",
        "course ",
        "class ",
        "study",
    ];
    if KEYS.iter().any(|k| g.contains(k)) {
        return true;
    }
    // Glued course codes: ENGL1140, CS2110
    for word in goals.split(|c: char| !c.is_alphanumeric()) {
        let w = word.to_lowercase();
        let alpha: String = w.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let digits: String = w.chars().skip(alpha.len()).collect();
        if alpha.len() >= 2
            && alpha.len() <= 6
            && digits.len() >= 3
            && digits.len() <= 5
            && digits.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    // Spaced course codes: "ENGL 1140"
    let tokens: Vec<&str> = goals.split_whitespace().collect();
    for pair in tokens.windows(2) {
        let a = pair[0];
        let b = pair[1].trim_matches(|c: char| !c.is_ascii_digit());
        if a.len() >= 2
            && a.len() <= 6
            && a.chars().all(|c| c.is_ascii_alphabetic())
            && b.len() >= 3
            && b.len() <= 5
            && b.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn guess_youtube_context(blob: &str, page_text: &str, goals: &str) -> Option<bool> {
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
        "lab report",
        "prelab",
        "pre-lab",
        "pre lab",
        "how to write",
        "writing a",
        "workshop",
        "seminar",
        "mooc",
        "coursera",
        "edx",
    ];
    // Entertainment / music / meme signals — match titles like
    // "KSI - Thick of it, but with NO MUSIC" without waiting on the LLM.
    const ENTERTAIN: &[&str] = &[
        "music video",
        "official video",
        "official audio",
        "official mv",
        "no music",
        "without music",
        "lyrics",
        "lyric video",
        "karaoke",
        "remix",
        "sped up",
        "nightcore",
        "instrumental",
        "hip hop",
        "hip-hop",
        "rapping",
        "rapper",
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
        "live performance",
        "stand-up",
        "stand up",
        "vevo",
    ];
    // Gaming-flavored entertainment — still off by default, but allowed when goals name the game.
    const GAMING_FUN: &[&str] = &[
        "minecraft",
        "fortnite",
        "gameplay",
        "playthrough",
        "speedrun",
        "let's play",
        "lets play",
    ];

    // Title + URL only for study/goal evidence — YouTube sidebars often contain
    // "lecture"/"tutorial" recommendations that must not green-light entertainment.
    let head = title_url_head(page_text);
    let study_hit = STUDY.iter().any(|k| head.contains(k));
    // Entertainment may also appear in a short OCR/page excerpt when the title is sparse.
    let fun_hit = ENTERTAIN.iter().any(|k| head.contains(k) || blob.contains(k))
        || music_or_rap_signal(&head)
        || music_or_rap_signal(blob);
    let gaming_fun = GAMING_FUN
        .iter()
        .any(|k| head.contains(k) || blob.contains(k));

    // Goal keywords appearing in the video title/URL → strong on-task signal.
    // Do not treat bare "school" / tab-group words as goals.
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| head.contains(k))
        .count();

    // Goal-named gameplay/playtest first — "walkthrough" study chrome must not force
    // unclear/Hard when the session is explicitly a game-design assignment.
    if gaming_fun && goal_overlap >= 1 {
        return Some(true);
    }
    if fun_hit && !study_hit {
        return Some(false);
    }
    if goal_overlap >= 1 && !fun_hit {
        return Some(true);
    }
    if study_hit && !fun_hit {
        return Some(true);
    }
    None
}

/// Academic papers / PDFs / scholar: "academic" alone is not enough — must overlap goals.
fn guess_reading_context(blob: &str, page_text: &str, goals: &str) -> Option<bool> {
    let head = title_url_head(page_text);
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| head.contains(k))
        .count();
    if goal_overlap >= 1 {
        return Some(true);
    }
    // Paper/PDF chrome without goal match → off-topic for this session.
    if looks_like_academic_reading(&head) || looks_like_academic_reading(blob) {
        return Some(false);
    }
    None
}

fn looks_like_academic_reading(s: &str) -> bool {
    const MARKERS: &[&str] = &[
        "arxiv",
        "abstract",
        "doi.org",
        "doi:",
        ".pdf",
        "scholar",
        "pubmed",
        "proceedings",
        "journal of",
        "researchgate",
        "biorxiv",
        "preprint",
        "semanticscholar",
        "acm digital",
        "ieee xplore",
        "full text pdf",
        "cite this",
    ];
    MARKERS.iter().any(|m| s.contains(m))
}

/// Steam / Epic / game clients: on-task only when title/URL overlaps session goals.
fn guess_gaming_context(page_text: &str, goals: &str) -> Option<bool> {
    let head = title_url_head(page_text);
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| head.contains(k))
        .count();
    if goal_overlap >= 1 {
        return Some(true);
    }
    // Gaming surface with no goal match → interrupt (play/store is off-task by default).
    Some(false)
}

/// First two non-empty lines of page_text are title + URL from `page_context_blob`.
fn title_url_head(page_text: &str) -> String {
    let head: String = page_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    head.replace("tab group", " ").replace("school group", " ")
}

/// Music/rap chrome that substring lists miss (word boundaries, "Artist - Track" + music cues).
fn music_or_rap_signal(blob: &str) -> bool {
    // Word-ish tokens: "rap", "raps", "song", "songs", "mv" (not "map", "songwriting" study edge).
    for token in blob.split(|c: char| !c.is_alphanumeric()) {
        if matches!(token, "rap" | "raps" | "song" | "songs" | "mv" | "mvs" | "beat" | "beats")
        {
            return true;
        }
    }
    false
}

fn guess_discord_context(blob: &str, goals: &str) -> Option<bool> {
    const STUDY: &[&str] = &[
        "homework",
        "study",
        "lecture",
        "office hours",
        "tutoring",
        "homework help",
        "exam",
        "quiz",
        "problem set",
        "pset",
        "lab ",
        "discussion",
        "oh ",
        "ta help",
        "course",
    ];
    // Pure social spam — never rescued by goal word overlap in channel chrome.
    const MEME_OFF: &[&str] = &["meme", "memes", "nft", "anime", "shitpost", "general spam"];
    // Gaming channels are contextual: off by default, on-task when goals name the game.
    const GAME_OFF: &[&str] = &[
        "gaming",
        "game night",
        "valorant",
        "fortnite",
        "minecraft",
    ];
    let study_hit = STUDY.iter().any(|k| blob.contains(k));
    let meme_hit = MEME_OFF.iter().any(|k| blob.contains(k));
    let game_hit = GAME_OFF.iter().any(|k| blob.contains(k));
    let off_hit = meme_hit || game_hit;
    let goal_overlap = goal_keywords(goals)
        .into_iter()
        .filter(|k| blob.contains(k.as_str()))
        .count();
    // Goal match wins over gaming chrome (playtest / game-design assignments).
    if goal_overlap >= 1 && !meme_hit {
        return Some(true);
    }
    if study_hit && !off_hit {
        return Some(true);
    }
    if off_hit && !study_hit {
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

fn hard_app_label(app_name: &str) -> Option<&'static str> {
    let app = app_name.to_lowercase();
    // Always-off native apps (Instagram-class / shopping-adjacent / texting / streaming).
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

/// Contextual native apps: never hard-nag from app name alone — use window title + goals.
fn contextual_app_label(app_name: &str) -> Option<&'static str> {
    let app = app_name.to_lowercase();
    if app.contains("discord") {
        Some("discord")
    } else if app == "steam" || app.contains("steam helper") || app.contains("epic games") {
        Some("gaming")
    } else if app == "preview"
        || app.contains("acrobat")
        || app.contains("adobe reader")
        || app.contains("pdf expert")
    {
        Some("reading")
    } else {
        None
    }
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
        // Academic / papers — contextual (must match goals; not auto-clear).
        ("arxiv.org", "reading"),
        ("scholar.google", "reading"),
        ("pubmed.ncbi", "reading"),
        ("semanticscholar.org", "reading"),
        ("biorxiv.org", "reading"),
        ("medrxiv.org", "reading"),
        ("researchgate.net", "reading"),
        ("jstor.org", "reading"),
        ("sciencedirect.com", "reading"),
        ("ieeexplore.ieee", "reading"),
        ("dl.acm.org", "reading"),
        ("acm.org/doi", "reading"),
        ("nature.com", "reading"),
        ("springer.com", "reading"),
        ("wiley.com", "reading"),
        ("ssrn.com", "reading"),
        ("doi.org", "reading"),
        // LMS / course portals — contextual screen (goal-overlap → Clear; never youtube).
        ("canvas.", "screen"),
        ("instructure.com", "screen"),
        ("blackboard.", "screen"),
        ("blackboard.com", "screen"),
        ("brightspace.", "screen"),
        ("schoology.com", "screen"),
        // Gaming storefronts / launchers in-browser — contextual via goals.
        ("store.steampowered", "gaming"),
        ("steamcommunity.com", "gaming"),
        ("steampowered.com", "gaming"),
        ("epicgames.com", "gaming"),
        ("store.epicgames", "gaming"),
    ];
    for (needle, label) in RULES {
        if text.contains(needle) {
            return Some(label);
        }
    }
    // Browser PDF tabs (file.pdf / .../pdf) — treat as reading, not auto-clear.
    if text.contains(".pdf") || text.contains("/pdf") || text.contains("filetype=pdf") {
        return Some("reading");
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
    // Unit tests must stay hermetic — no live Safari/Chrome AppleScript.
    #[cfg(not(test))]
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

#[allow(dead_code)]
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

/// Whether macOS Accessibility (AX) is granted for this process.
#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    // SAFETY: plain C call; reads TCC Accessibility grant for this binary.
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted() -> bool {
    true
}

/// Show the system Accessibility prompt (opens Settings when the user confirms).
#[cfg(target_os = "macos")]
fn request_accessibility_prompt() -> bool {
    use std::ffi::c_void;

    type CfTypeRef = *const c_void;
    type CfDictionaryRef = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CfDictionaryRef) -> bool;
        static kAXTrustedCheckOptionPrompt: CfTypeRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const CfTypeRef,
            values: *const CfTypeRef,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CfDictionaryRef;
        fn CFRelease(cf: CfTypeRef);
        static kCFBooleanTrue: CfTypeRef;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
    }

    // SAFETY: symbols are process-global CoreFoundation / AX constants; dictionary
    // holds one boolean option and is released before return.
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let opts = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );
        if opts.is_null() {
            return AXIsProcessTrustedWithOptions(std::ptr::null());
        }
        let trusted = AXIsProcessTrustedWithOptions(opts);
        CFRelease(opts);
        trusted
    }
}

#[cfg(target_os = "macos")]
fn open_privacy_pane(pane: &str) {
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
    let _ = Command::new("open").arg(url).spawn();
}

/// Trigger the Automation (Apple Events) prompt for System Events, if needed.
#[cfg(target_os = "macos")]
fn probe_system_events_automation() -> Result<(), String> {
    // Minimal target — first call shows “Waypoint wants to control System Events”.
    osascript(r#"tell application "System Events" to get name"#).map(|_| ())
}

/// Prompt Accessibility + Automation before lock-in, then verify frontmost works.
///
/// Call this before spawning the coach loop so the session never starts with a
/// dead focus scanner and a late “Can’t read screen focus” note.
pub fn ensure_focus_permissions() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !accessibility_trusted() {
            let trusted = request_accessibility_prompt();
            if !trusted && !accessibility_trusted() {
                open_privacy_pane("Privacy_Accessibility");
                return Err(
                    "Allow Accessibility for Waypoint in System Settings → Privacy & Security → Accessibility, then quit and reopen Waypoint before launching."
                        .into(),
                );
            }
        }

        if let Err(e) = probe_system_events_automation() {
            open_privacy_pane("Privacy_Automation");
            return Err(format!(
                "Allow Automation for Waypoint → System Events in System Settings → Privacy & Security → Automation, then try again. ({e})"
            ));
        }

        frontmost_info().map(|_| ()).map_err(|e| {
            open_privacy_pane("Privacy_Accessibility");
            format!(
                "Can’t read screen focus yet ({e}). Allow Accessibility + Automation (System Events) for Waypoint in System Settings, then quit and reopen the app."
            )
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_entertainment_ksi_no_music_is_off_task() {
        let title = "KSI - Thick of it, but with NO MUSIC - YouTube";
        let url = "https://www.youtube.com/watch?v=SbTT1f3xZVg";
        let page = format!("{title}\n{url}\nKSI Rapping");
        assert_eq!(
            local_context_guess("youtube", &page, "ENGL 1140 discussion; BIOMG 1350 quiz"),
            Some(false)
        );
    }

    #[test]
    fn youtube_lecture_matching_goals_is_on_task() {
        let page = "MIT 6.006 Introduction to Algorithms, Lecture 1 - YouTube\nhttps://www.youtube.com/watch?v=abc\nlecture algorithms";
        assert_eq!(
            local_context_guess("youtube", page, "algorithms / 6.006 problem set"),
            Some(true)
        );
    }

    #[test]
    fn school_tab_group_alone_does_not_mark_entertainment_on_task() {
        // Tab-group chrome must not override clear music/entertainment titles.
        let page = "KSI - Thick of it, but with NO MUSIC\nhttps://www.youtube.com/watch?v=x\nSchool tab group";
        assert_eq!(
            local_context_guess("youtube", page, "ENGL / BIOMG schoolwork"),
            Some(false)
        );
    }

    #[test]
    fn surface_policy_instagram_always_off_youtube_contextual() {
        assert_eq!(surface_policy("instagram"), SurfacePolicy::AlwaysOff);
        assert_eq!(surface_policy("shopping"), SurfacePolicy::AlwaysOff);
        assert_eq!(surface_policy("texting"), SurfacePolicy::AlwaysOff);
        assert_eq!(surface_policy("youtube"), SurfacePolicy::Contextual);
        assert_eq!(surface_policy("discord"), SurfacePolicy::Contextual);
        assert_eq!(surface_policy("reading"), SurfacePolicy::Contextual);
        assert_eq!(surface_policy("gaming"), SurfacePolicy::Contextual);
        assert_eq!(surface_policy("screen"), SurfacePolicy::Contextual);
        assert_eq!(surface_policy("editor"), SurfacePolicy::Contextual);
    }

    #[test]
    fn focused_canvas_with_bg_youtube_is_not_youtube_hard() {
        // Bug repro: Safari on Canvas + background YouTube tab must not say "youtube".
        let info = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "Week 7: Discussion - ENGL 1140".into(),
            url: "https://canvas.cornell.edu/courses/73512/discussion_topics/912".into(),
        };
        let bg = [BrowserTab {
            url: "https://www.youtube.com/watch?v=SbTT1f3xZVg".into(),
            title: "KSI - Thick of it, but with NO MUSIC - YouTube".into(),
        }];
        assert_eq!(
            classify_url_host(&info.url.to_lowercase()),
            Some("screen")
        );
        let event = decide_focus(info, "Finish ENGL 1140 work", &bg).expect("decide");
        match event {
            FocusEvent::Hard(hit) => {
                panic!(
                    "expected Clear/NeedsJudgment for Canvas, got Hard {} ({})",
                    hit.label, hit.detail
                );
            }
            FocusEvent::Clear(_) => {}
            FocusEvent::NeedsJudgment { kind, .. } => {
                assert_ne!(kind, "youtube", "Canvas must not be labeled youtube");
                assert_ne!(kind, "video");
            }
        }
    }

    #[test]
    fn focused_youtube_entertainment_still_hard() {
        let info = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "KSI - Thick of it, but with NO MUSIC - YouTube".into(),
            url: "https://www.youtube.com/watch?v=SbTT1f3xZVg".into(),
        };
        let event = decide_focus(info, "Finish ENGL 1140 work", &[]).expect("decide");
        match event {
            FocusEvent::Hard(hit) => assert_eq!(hit.label, "youtube"),
            other => panic!("expected youtube Hard, got {other:?}"),
        }
    }

    #[test]
    fn editor_with_engl_goals_not_clear() {
        let info = FrontmostInfo {
            app_name: "Cursor".into(),
            window_title: "frontmost.rs — BigRedHacksProject".into(),
            url: String::new(),
        };
        assert_eq!(
            local_context_guess(
                "editor",
                "frontmost.rs — BigRedHacksProject\nCursor",
                "Finish ENGL 1140 work"
            ),
            Some(false)
        );
        let event = decide_focus(info, "Finish ENGL 1140 work", &[]).expect("decide");
        match event {
            FocusEvent::Clear(_) => panic!("editor must not Clear for ENGL coursework goals"),
            FocusEvent::Hard(hit) => assert_eq!(hit.label, "editor"),
            FocusEvent::NeedsJudgment { kind, .. } => assert_eq!(kind, "editor"),
        }
    }

    #[test]
    fn screen_unclear_is_needs_judgment_not_hard() {
        let info = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "Random docs page".into(),
            url: "https://example.com/notes".into(),
        };
        let event = decide_focus(info, "Finish ENGL 1140 work", &[]).expect("decide");
        match event {
            FocusEvent::NeedsJudgment { kind, .. } => assert_eq!(kind, "screen"),
            FocusEvent::Hard(hit) => panic!("screen unclear must not Hard ({})", hit.label),
            FocusEvent::Clear(_) => panic!("unclear screen without goal overlap should not Clear"),
        }
    }

    #[test]
    fn off_topic_arxiv_paper_is_off_task() {
        assert_eq!(
            classify_url_host("https://arxiv.org/abs/1706.03762"),
            Some("reading")
        );
        let page = "Attention Is All You Need\nhttps://arxiv.org/pdf/1706.03762.pdf\nAbstract We propose a new simple network architecture";
        assert_eq!(
            local_context_guess("reading", page, "ENGL 1140 discussion post"),
            Some(false)
        );
    }

    #[test]
    fn on_topic_paper_title_matching_goals_is_on_task() {
        let page = "ENGL 1140 Peer Review Strategies in First-Year Composition - Google Scholar\nhttps://scholar.google.com/scholar?q=engl+1140+peer+review\n";
        assert_eq!(
            classify_url_host("https://scholar.google.com/scholar?q=engl+1140"),
            Some("reading")
        );
        assert_eq!(
            local_context_guess("reading", page, "ENGL 1140 discussion post"),
            Some(true)
        );
    }

    #[test]
    fn on_topic_game_matching_goals_is_on_task() {
        assert_eq!(
            classify_url_host("https://store.steampowered.com/app/367520/Hollow_Knight/"),
            Some("gaming")
        );
        let page = "Hollow Knight on Steam\nhttps://store.steampowered.com/app/367520/Hollow_Knight/\n";
        assert_eq!(
            local_context_guess(
                "gaming",
                page,
                "playtest Hollow Knight for game design assignment"
            ),
            Some(true)
        );
    }

    #[test]
    fn off_topic_game_is_off_task() {
        let page = "Counter-Strike 2 on Steam\nhttps://store.steampowered.com/app/730/CounterStrike_2/\n";
        assert_eq!(
            local_context_guess("gaming", page, "ENGL 1140 discussion post"),
            Some(false)
        );
    }

    #[test]
    fn youtube_gameplay_on_topic_for_game_design_goals() {
        let page = "Hollow Knight Path of Pain gameplay walkthrough - YouTube\nhttps://www.youtube.com/watch?v=x\n";
        assert_eq!(
            local_context_guess(
                "youtube",
                page,
                "playtest Hollow Knight for game design class"
            ),
            Some(true)
        );
    }

    #[test]
    fn discord_gaming_channel_on_topic_when_goals_match() {
        assert_eq!(
            local_context_guess(
                "discord",
                "#hollow-knight | playtest notes",
                "Hollow Knight playtest for game design"
            ),
            Some(true)
        );
        // Gaming chrome without goal overlap stays off-task.
        assert_eq!(
            local_context_guess("discord", "#minecraft | gaming", "ENGL 1140 discussion"),
            Some(false)
        );
    }

    #[test]
    fn discord_study_vs_meme_from_title() {
        assert_eq!(
            local_context_guess(
                "discord",
                "#homework-help | ENGL 1140 discussion",
                "ENGL 1140 discussion post"
            ),
            Some(true)
        );
        assert_eq!(
            local_context_guess("discord", "#memes | shitpost central", "ENGL 1140 discussion"),
            Some(false)
        );
    }

    #[test]
    fn classify_url_host_instagram_and_youtube() {
        assert_eq!(
            classify_url_host("https://www.instagram.com/reel/xyz"),
            Some("instagram")
        );
        assert_eq!(
            classify_url_host("https://www.youtube.com/watch?v=1"),
            Some("youtube")
        );
        assert_eq!(
            classify_url_host("https://www.youtube.com/shorts/abc123"),
            Some("youtube")
        );
        assert_eq!(
            classify_url_host("https://youtu.be/abc123"),
            Some("youtube")
        );
        assert_ne!(
            classify_url_host("https://www.youtube.com/shorts/abc123"),
            Some("instagram")
        );
    }

    #[test]
    fn focus_supports_youtube_only_when_host_or_chrome_matches() {
        let canvas = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "ENGL 1140 Discussion".into(),
            url: "https://canvas.cornell.edu/courses/1/discussion_topics/2".into(),
        };
        assert!(
            !focus_supports_distraction_label("youtube", &canvas),
            "Canvas must not corroborate youtube"
        );

        let yt = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "KSI - Thick of it - YouTube".into(),
            url: "https://www.youtube.com/watch?v=SbTT1f3xZVg".into(),
        };
        assert!(focus_supports_distraction_label("youtube", &yt));

        let title_only = FrontmostInfo {
            app_name: "Safari".into(),
            window_title: "Lecture 3 - YouTube".into(),
            url: String::new(),
        };
        assert!(focus_supports_distraction_label("youtube", &title_only));
    }

    #[test]
    fn distraction_coach_line_background_youtube_is_not_focused_copy() {
        let hit = DistractionHit {
            label: "youtube",
            detail: "Safari · youtube.com".into(),
            focused: false,
        };
        let line = distraction_coach_line(&hit).to_lowercase();
        assert!(
            line.contains("background"),
            "background youtube must say background, got: {line}"
        );
        let focused = DistractionHit {
            label: "youtube",
            detail: "Safari · youtube.com".into(),
            focused: true,
        };
        let focused_line = distraction_coach_line(&focused).to_lowercase();
        assert!(
            !focused_line.contains("background"),
            "focused youtube must not say background: {focused_line}"
        );
    }

    #[test]
    fn youtube_shorts_entertainment_is_hard_off_task() {
        let page = "Want to learn how to code? #shorts - YouTube\nhttps://www.youtube.com/shorts/xyz\nShorts";
        assert_eq!(
            local_context_guess("youtube", page, "coding project"),
            Some(false)
        );
        let hit = DistractionHit {
            label: "youtube",
            detail: "YouTube Shorts".into(),
            focused: true,
        };
        let line = distraction_coach_line(&hit);
        assert!(line.to_lowercase().contains("youtube"));
        assert!(!line.to_lowercase().contains("instagram"));
    }

    #[test]
    fn youtube_sidebar_lecture_does_not_greenlight_music() {
        // Body recommendations must not override a clear entertainment title.
        let page = "KSI - Thick of it, but with NO MUSIC - YouTube\nhttps://www.youtube.com/watch?v=x\nUp next: MIT OCW calculus lecture tutorial explained";
        assert_eq!(
            local_context_guess("youtube", page, "calculus exam prep"),
            Some(false)
        );
    }

    #[test]
    fn youtube_unclear_title_is_not_study() {
        // No study/entertainment keywords → unclear (caller hard-nags YouTube).
        let page = "Random Cat Video - YouTube\nhttps://www.youtube.com/watch?v=abc\nHome Recommended";
        assert_eq!(
            local_context_guess("youtube", page, "ENGL 1140 discussion"),
            None
        );
    }

    #[test]
    fn youtube_goal_word_only_in_sidebar_is_not_study() {
        let page = "Funny prank compilation - YouTube\nhttps://www.youtube.com/watch?v=abc\nRelated: ENGL 1140 discussion lecture";
        assert_eq!(
            local_context_guess("youtube", page, "ENGL 1140 discussion"),
            Some(false)
        );
    }
}
