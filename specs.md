# Waypoint

Study companion for a single student. It watches the work session, remembers who the student is across sessions, reads assignment due dates from Google Calendar, and can either advise on the current task or take the next step with them.

The desktop app talks to Gemini. Durable facts live on the Waypoint API. The client never writes the database itself.

## Authentication

On launch the app signs in through the backend and receives:

- The Waypoint user
- The memory profile (interests, proficiencies, goals, pace, priorities, interaction preferences)
- Google Calendar connection status

Sign-in uses identity scopes only. Calendar is a separate connect action so a student can use Waypoint without granting calendar access.

## Memory

One profile per user, stored on the backend and loaded into every Gemini call as a short profile card. The card is what the model sees. Raw pace history stays on the server.

| Field | What it stores | How it changes |
| --- | --- | --- |
| Interests | Topics they care about, used for analogies | Student says so, or confirms an inference |
| Proficiencies | A topic plus `learning`, `comfortable`, or `strong` | Student states a level, or it moves one step after a session |
| Long-term goals | Semester or career aims, separate from today's task | Student says so |
| Pace | Median minutes for a kind of problem | Append-only samples at session end |
| Priorities | Ordered list of what matters this week | Student says so; deadlines can suggest an order |
| Interaction preferences | Tone, advise vs pair, when to interrupt, voice, check-in interval | Student says so, or sets them in the profile screen |

Pace is "how fast you work for each problem." A sample is `{ topic, problem, planned_minutes, actual_minutes, outcome }` where outcome is `finished`, `partial`, or `abandoned`. The card shows the median of the last 8 finished samples for that topic, plus the sample count. When the student names a duration, the model compares it to that median.

Writes go through a `remember` tool. The app shows a one-line confirmation ("Saved: short hints") with undo. Inferred pace and proficiency updates are saved at session end without a prompt; everything else waits for the student to say it or accept it.

Caps, so the card stays small: 12 interests, 20 proficiencies, 8 goals, 8 priorities. Newer explicit statements replace the same item. Older inferred items drop first.

If the API is unreachable, the app uses the last fetched card and skips writes for that session.

## Calendar

Connect grants `calendar.events` on the primary calendar. The backend holds the Google refresh token and is the only caller of the Calendar API.

**Read.** Before a chat reply or a lock-in decision, the app loads the next 14 days, already classified:

- **Deadline** — all-day, or the title matches due, deadline, submit, exam, quiz, prelim, midterm, final, hw, pset, or assignment. This is how assignment due dates show up (including a Canvas calendar synced into Google). Waypoint does not talk to Canvas directly.
- **Block** — timed events (class, a study block, anything else).

Deadlines outrank blocks when the model plans the day. The profile's priorities break ties between deadlines.

**Write.** The model may propose an event. It cannot create one itself. The app shows a card with title, time, and Add / Dismiss. Add calls the backend. Created events are tagged `waypoint` and are either:

- a **study block** (timed), or
- a **deadline** (all-day, title kept as the student said it).

The model may move or delete only events it created. Other calendar events are read-only.

A spoken due date ("HW3 is due Friday") is a proposal, not an automatic insert.

## Working on a task

Every lock-in has one active task and a mode. The mode starts from `interaction.default_mode` (`advise`, `pair`, or `ask`). Voice can switch it: "just advise me" or "work on this with me."

**Advise.** Help without doing the work. One hint or one question, aimed at the student's proficiency. No solution, no full code, no finished proof. If they ask for the answer, offer a stronger hint instead. Use pace and the next deadline to size the hint ("you usually need 35 minutes; you have 20 before lecture").

**Pair.** Do the next step with them, then stop. The step is one artifact: a function stub, the next proof claim, the next outline node, or the setup of the next sub-question. The latest screenshot and their last utterance are the context. After the step, wait. Review what they do next and point at the first mistake. A full solution is allowed only when they explicitly ask for the answer.

`ask` means the model asks which mode once at task start, then stays there.

## Voice input

The student can talk at any time to set the agenda, set a priority, start a break, end the session, switch advise/pair, ask for a screenshot check, or tell Waypoint something to remember.

## Screen input

Every 15 seconds a screenshot is taken and judged against the active task. On task: stay quiet. Off task: the interrupt preference decides whether to speak. Pair mode also uses the latest screenshot as the thing being worked on.

## Camera input

Presage SmartSpectra reads the camera and submits a wellness signal for the session summary. Lock-in still runs if the camera is denied.

## Decision making

The app uses an ephemeral Gemini credential from the backend. Gemini picks an action with tools:

- `speak` — Grok Voice speaks the line
- `setTimer` — call Gemini again when the timer ends
- `updateGoal` — replace the active task title
- `setTaskMode` — `advise` or `pair`
- `remember` — patch the profile (interests, proficiency, goal, priority, interaction preference)
- `recordPace` — one pace sample for the problem just attempted
- `proposeEvent` — calendar card; nothing is written until the student confirms
- Do nothing

Advise vs pair changes what a spoken reply is allowed to contain. It is a reply policy on top of these tools, not a separate model.

## Session end

Tell them how the session went:

- Time spent and breaks
- Attention, from the screenshot judgments
- Whether the pace sample was faster or slower than their median for that topic
- The next deadline this task was for, if one was linked

The summary and the pace sample are saved through the backend.

When session notes are on, that same final review shows the lock-in note. The student can press **Concept map**. The desktop sends the note markdown to `POST /v1/concept-map`. The API asks Grok Imagine for one diagram of what the note says they learned, and the image appears on the review. The map is not stored with the note. Sign-in is required, because the xAI key stays on the API.

## Tech stack

- Desktop: Rust / Tauri
- Model: Gemini, with tools
- Voice out: Grok Voice
- Final-review concept map: Grok Imagine (`grok-imagine-image-2.0`) via the API
- Memory, auth, calendar: Waypoint API

## Example

00:00 — Profile already loaded. Interests include systems. Heaps are `learning`. Median pace for heap problems is 40 minutes. Interaction: brief, advise, interrupt only after sustained low focus. Next deadline: CS 4820 HW3, Friday.

Student: "I want to finish problem 3 in thirty minutes."

Gemini: that is tight versus a 40 minute median. `proposeEvent` for a 19:00–19:40 study block. `updateGoal("Problem 3")`. `setTimer(1800)`. Speak: "You usually need about forty minutes on these. I put a forty-minute block on your calendar — dismiss it if you want the half hour. Starting the timer."

05:00 — focus stays high. Silent.

11:45 — sustained low focus.

Speak: "You seem to have slowed down. Did you hit a blocker?"

Student: "Yeah, I'm stuck on the priority queue. Work on it with me."

`setTaskMode(pair)`. `remember` proficiency `priority queues = learning`. `setTimer(600)`. Speak the next step only: the heap invariant, then a stub for `push`, and stop.

15:00 — screenshot is the editor with `push` in progress. Focus high. Silent.

16:00 — screenshot is Youtube Shorts. Speak: "That doesn't look like the priority queue."

17:00 — screenshot is a lecture clip on heaps. Silent.

30:00 — timer ends. Student finished `push`.

`recordPace(topic: "heaps", problem: "HW3 problem 3", planned: 30, actual: 30, outcome: finished)`.

Summary: 30 minutes, no break, attention recovered after the hint, pace sample saved (faster than the 40 minute median). Next deadline is still Friday's HW3.
