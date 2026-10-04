//! Readable lock-in note. Text sources only — no screenshots, no vision loop.
//! Wording comes from Copilot chat on the API (`POST /v1/gemini/chat`), which
//! picks Gemini or Ollama. A deterministic note is the fallback.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::api;
use crate::auth;
use crate::config::AppConfig;
use crate::session::LockInSession;
use crate::AppState;

/// Organization rules for the note. Kind is chosen in code and named in the user message.
pub const NOTE_SYSTEM_PROMPT: &str = "\
You write one readable Markdown session note for a Waypoint lock-in. Use only the sources in the user message. Do not invent facts, files, courses, or decisions that are not there.

The user message names the kind. Follow it.
- study: the user narrated study in their own words (explaining a concept, teaching it back, \"let me explain\"). Sections, in order: a Markdown title, What I was learning, In my own words, Gaps / shaky parts, Next.
- devlog: the user was building, coding, or debugging (editor, terminal, repo work, coding modality, implementation goals). Sections, in order: a Markdown title, What I worked on, Decisions, Stuck on, Next.
If the sources are mixed, the named kind is the dominant activity from goals, narration, and screen summaries. Follow that kind.

Coach prompt lines are context only. Never treat them as the user's words.
Output the Markdown note only. No JSON, and no code fence around the note. If a section has no evidence, write \"Not captured.\"";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Study,
    Devlog,
}

impl NoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Devlog => "devlog",
        }
    }

    fn section_markers(self) -> &'static [&'static str] {
        match self {
            Self::Study => &["What I was learning", "In my own words"],
            Self::Devlog => &["What I worked on", "Decisions"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoteSources {
    pub goals: String,
    pub modality: String,
    pub user_utterances: Vec<String>,
    pub screen_lines: Vec<String>,
    pub coach_prompts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NoteJob {
    pub session_id: String,
    pub started_at: String,
    pub sources: NoteSources,
}

impl NoteJob {
    pub fn from_session(session: &LockInSession, user_utterances: Vec<String>) -> Self {
        Self {
            session_id: session.id.clone(),
            started_at: session.started_at.clone(),
            sources: NoteSources {
                goals: session.goals.clone(),
                modality: session.modality.clone(),
                user_utterances,
                screen_lines: session.screen_log.clone(),
                coach_prompts: session.prompts.iter().map(|p| p.text.clone()).collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SessionNoteReady {
    session_id: String,
    kind: String,
    markdown: String,
}

/// Dominant activity from goals, the user's narration, and screen summaries.
/// Feynman / teach-back phrases in the user's words weigh heavily toward `study`.
/// Editor, terminal, repo, and implementation language weigh toward `devlog`.
pub fn choose_kind(sources: &NoteSources) -> NoteKind {
    let user = sources.user_utterances.join("\n").to_lowercase();
    let goals = sources.goals.to_lowercase();
    let modality = sources.modality.to_lowercase();
    let screen = sources.screen_lines.join("\n").to_lowercase();

    let mut study = 0i32;
    let mut dev = 0i32;

    const FEYNMAN: &[&str] = &[
        "let me explain",
        "i'll explain",
        "i will explain",
        "let me teach",
        "teach it back",
        "teaching it back",
        "in my own words",
        "the idea is",
        "what this means",
        "so basically",
        "feynman",
    ];
    for phrase in FEYNMAN {
        if user.contains(phrase) {
            study += 8;
        }
    }

    const STUDY_WORDS: &[&str] = &[
        "learn",
        "learning",
        "study",
        "studying",
        "lecture",
        "chapter",
        "homework",
        "theorem",
        "concept",
        "exam",
        "quiz",
        "flashcard",
        "reading",
        "understand",
        "definition",
        "explain",
    ];
    const DEV_WORDS: &[&str] = &[
        "code",
        "coding",
        "debug",
        "debugging",
        "bug",
        "implement",
        "implementation",
        "refactor",
        "terminal",
        "compile",
        "compiler",
        "editor",
        "repo",
        "commit",
        "function",
        "rust",
        "typescript",
        "javascript",
        "python",
        "cargo",
        "npm",
        "build",
        "pull request",
        "stack trace",
    ];
    const DEV_APPS: &[&str] = &[
        "cursor",
        "visual studio",
        "vs code",
        "vscode",
        "terminal",
        "iterm",
        "xcode",
        "intellij",
        "webstorm",
        "neovim",
        "warp",
        "sublime",
        "android studio",
    ];

    study += word_hits(&user, STUDY_WORDS) * 2;
    dev += word_hits(&user, DEV_WORDS) * 2;
    study += word_hits(&goals, STUDY_WORDS);
    dev += word_hits(&goals, DEV_WORDS);
    study += word_hits(&screen, STUDY_WORDS);
    dev += word_hits(&screen, DEV_WORDS);
    dev += word_hits(&screen, DEV_APPS) * 2;

    // Session modality today is paper / mixed / computer. A coding label still counts.
    if modality.contains("coding") || modality == "code" {
        dev += 3;
    }

    if dev > study {
        NoteKind::Devlog
    } else {
        NoteKind::Study
    }
}

fn word_hits(text: &str, words: &[&str]) -> i32 {
    words.iter().filter(|w| text.contains(*w)).count() as i32
}

pub fn fallback_markdown(sources: &NoteSources, kind: NoteKind) -> String {
    let title = note_title(&sources.goals);
    match kind {
        NoteKind::Study => {
            let learning = nonempty_or(&sources.goals, "Not captured.");
            let words = bullets_or(&sources.user_utterances, "Not captured.");
            let gaps = bullets_or(&gap_lines(&sources.screen_lines), "Not captured.");
            let next = bullets_or(
                &lines_mentioning(&sources.user_utterances, &["next"]),
                "Not captured.",
            );
            format!(
                "# {title}\n\n## What I was learning\n{learning}\n\n## In my own words\n{words}\n\n## Gaps / shaky parts\n{gaps}\n\n## Next\n{next}\n"
            )
        }
        NoteKind::Devlog => {
            let mut worked = Vec::new();
            if !sources.goals.trim().is_empty() {
                worked.push(sources.goals.trim().to_string());
            }
            worked.extend(sources.screen_lines.iter().cloned());
            let worked = bullets_or(&worked, "Not captured.");
            let decisions = decision_lines(&sources.user_utterances);
            let decisions = bullets_or(&decisions, "Not captured.");
            let mut stuck_src = sources.user_utterances.clone();
            stuck_src.extend(sources.screen_lines.iter().cloned());
            let stuck = bullets_or(
                &lines_mentioning(&stuck_src, &["stuck", "error", "bug", "fail", "debug"]),
                "Not captured.",
            );
            let next = bullets_or(
                &lines_mentioning(&sources.user_utterances, &["next", "todo"]),
                "Not captured.",
            );
            format!(
                "# {title}\n\n## What I worked on\n{worked}\n\n## Decisions\n{decisions}\n\n## Stuck on\n{stuck}\n\n## Next\n{next}\n"
            )
        }
    }
}

fn note_title(goals: &str) -> String {
    let line = goals
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("Session note");
    let clipped = clip_chars(line.trim(), 80);
    if clipped.is_empty() {
        "Session note".into()
    } else {
        clipped
    }
}

fn nonempty_or(text: &str, empty: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        empty.to_string()
    } else {
        trimmed.to_string()
    }
}

fn bullets_or(lines: &[String], empty: &str) -> String {
    let items: Vec<&str> = lines
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        empty.to_string()
    } else {
        items
            .into_iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn gap_lines(screen: &[String]) -> Vec<String> {
    lines_mentioning(
        screen,
        &[
            "distract", "off task", "off-task", "error", "stuck", "unsure", "shaky", "confused",
        ],
    )
}

fn decision_lines(utterances: &[String]) -> Vec<String> {
    let picked = lines_mentioning(
        utterances,
        &[
            "decided",
            "decision",
            "because",
            "chose",
            "instead",
            "going to",
            "i'll use",
            "i will use",
        ],
    );
    if picked.is_empty() {
        Vec::new()
    } else {
        picked
    }
}

fn lines_mentioning(lines: &[String], needles: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            let lower = line.to_lowercase();
            needles.iter().any(|n| lower.contains(n))
        })
        .cloned()
        .collect()
}

fn take_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars).collect()
    }
}

fn clip_chars(s: &str, max_chars: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        flat
    } else {
        let cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

fn user_message(sources: &NoteSources, kind: NoteKind) -> String {
    format!(
        "kind: {kind}\n\nGOALS:\n{goals}\n\nMODALITY:\n{modality}\n\nUSER'S OWN WORDS (companion turns):\n{user}\n\nSCREEN SUMMARIES (local app/title/URL, OCR, judge — text only):\n{screen}\n\nCOACH PROMPTS (context only, not the user's words):\n{coach}",
        kind = kind.as_str(),
        goals = nonempty_or(&sources.goals, "(none)"),
        modality = nonempty_or(&sources.modality, "(none)"),
        user = bullets_or(
            &sources
                .user_utterances
                .iter()
                .take(20)
                .map(|s| clip_chars(s, 500))
                .collect::<Vec<_>>(),
            "(none)",
        ),
        screen = bullets_or(
            &sources
                .screen_lines
                .iter()
                .map(|s| clip_chars(s, 240))
                .collect::<Vec<_>>(),
            "(none)",
        ),
        coach = bullets_or(
            &sources
                .coach_prompts
                .iter()
                .rev()
                .take(12)
                .map(|s| clip_chars(s, 200))
                .collect::<Vec<_>>(),
            "(none)",
        ),
    )
}

fn strip_wrapping_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_start = trimmed
        .strip_prefix("```markdown")
        .or_else(|| trimmed.strip_prefix("```md"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim();
    without_start
        .strip_suffix("```")
        .unwrap_or(without_start)
        .trim()
        .to_string()
}

fn accept_model_markdown(raw: &str, kind: NoteKind) -> Option<String> {
    let mut text = strip_wrapping_fence(raw);
    if text
        .lines()
        .next()
        .is_some_and(|line| line.trim().to_lowercase().starts_with("kind:"))
    {
        text = text
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
    }
    if text.chars().count() < 40 {
        return None;
    }
    let lower = text.to_lowercase();
    if kind
        .section_markers()
        .iter()
        .all(|marker| lower.contains(&marker.to_lowercase()))
    {
        Some(text)
    } else {
        None
    }
}

/// Copilot chat on the API. The desktop does not call Ollama or the lock-in model.
/// `Err` means the caller should use the fallback.
async fn write_with_models(
    cfg: &AppConfig,
    sources: &NoteSources,
    kind: NoteKind,
) -> Result<String, String> {
    if auth::load_tokens(cfg).is_none() {
        return Err("signed out".into());
    }
    let message = user_message(sources, kind);
    #[derive(serde::Deserialize)]
    struct ChatReply {
        content: String,
    }
    let body = serde_json::json!({
        "message": message,
        "system": NOTE_SYSTEM_PROMPT,
        "history": [],
    });
    let out: ChatReply =
        api::authed_json(cfg, reqwest::Method::POST, "/v1/gemini/chat", Some(&body)).await?;
    accept_model_markdown(&out.content, kind).ok_or_else(|| "chat note was not usable".into())
}

async fn upload_note(cfg: &AppConfig, job: &NoteJob, kind: NoteKind, markdown: &str) {
    if auth::load_tokens(cfg).is_none() {
        return;
    }
    let body = serde_json::json!({
        "session_id": job.session_id,
        "started_at": job.started_at,
        "ended_at": chrono::Utc::now().to_rfc3339(),
        "goals": take_chars(&job.sources.goals, 2_000),
        "kind": kind.as_str(),
        "markdown": take_chars(markdown, 32_000),
    });
    // Best-effort. Any 2xx counts; the response body is not part of the contract we rely on.
    if let Err(e) =
        api::authed_empty(cfg, reqwest::Method::POST, "/v1/session-notes", Some(&body)).await
    {
        tracing::warn!("session note upload: {e}");
    }
}

pub fn spawn_session_note(app: &AppHandle, job: NoteJob) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let cfg = app.state::<AppState>().config.lock().clone();
        let kind = choose_kind(&job.sources);
        let markdown = match write_with_models(&cfg, &job.sources, kind).await {
            Ok(md) => md,
            Err(e) => {
                tracing::warn!("session note model: {e}");
                fallback_markdown(&job.sources, kind)
            }
        };
        let _ = app.emit(
            "session-note",
            &SessionNoteReady {
                session_id: job.session_id.clone(),
                kind: kind.as_str().to_string(),
                markdown: markdown.clone(),
            },
        );
        upload_note(&cfg, &job, kind, &markdown).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(
        goals: &str,
        modality: &str,
        user: &[&str],
        screen: &[&str],
        coach: &[&str],
    ) -> NoteSources {
        NoteSources {
            goals: goals.into(),
            modality: modality.into(),
            user_utterances: user.iter().map(|s| (*s).to_string()).collect(),
            screen_lines: screen.iter().map(|s| (*s).to_string()).collect(),
            coach_prompts: coach.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn feynman_narration_is_study_and_fallback_uses_their_words() {
        let sources = sources(
            "biology chapter 3",
            "computer",
            &["let me explain photosynthesis in my own words"],
            &["Safari · textbook"],
            &["Heads up — instagram isn’t the mission."],
        );
        assert_eq!(choose_kind(&sources), NoteKind::Study);
        let md = fallback_markdown(&sources, NoteKind::Study);
        assert!(md.contains("## What I was learning"));
        assert!(md.contains("## In my own words"));
        assert!(md.contains("## Gaps / shaky parts"));
        assert!(md.contains("## Next"));
        assert!(md.contains("let me explain photosynthesis in my own words"));
        assert!(md.contains("biology chapter 3"));
        assert!(!md.contains("instagram isn’t the mission"));
        assert!(!md.contains("## What I worked on"));
    }

    #[test]
    fn coding_session_is_devlog_with_stuck_section() {
        let sources = sources(
            "implement the parser",
            "coding",
            &["stuck on the borrow checker"],
            &["Cursor · repo", "Terminal · cargo test"],
            &["Nice — this looks on track."],
        );
        assert_eq!(choose_kind(&sources), NoteKind::Devlog);
        let md = fallback_markdown(&sources, NoteKind::Devlog);
        assert!(md.contains("## What I worked on"));
        assert!(md.contains("implement the parser"));
        assert!(md.contains("Cursor · repo"));
        assert!(md.contains("## Decisions"));
        assert!(md.contains("## Stuck on"));
        assert!(md.contains("borrow checker"));
        assert!(md.contains("## Next"));
        assert!(md.contains("Not captured."));
        assert!(!md.contains("Nice — this looks on track."));
    }

    #[test]
    fn mixed_sources_follow_the_heavier_activity() {
        let study_heavy = sources(
            "review lecture notes",
            "computer",
            &["so basically the idea is recursion bottoms out"],
            &["Cursor · notes.md"],
            &[],
        );
        assert_eq!(choose_kind(&study_heavy), NoteKind::Study);

        let dev_heavy = sources(
            "debug the upload handler",
            "computer",
            &["the bug is a null check"],
            &["VS Code · api.ts", "Terminal · npm test"],
            &[],
        );
        assert_eq!(choose_kind(&dev_heavy), NoteKind::Devlog);
    }
}
