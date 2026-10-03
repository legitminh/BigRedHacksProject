# Frontend of PROJECT_NAME

## Function

Act as study companion for user

## Voice input

The user can talk at anytime to set work session agenda, priority, start break, end session, request screenshot.

## Screen input

Every 15 second, a screenshot is taken and analyzed to make sure the user is on task and provide further context on the task.

## Camera input

Presage's SmartSpectra SDK analyze camera feed and submit.

## Timer

The gemini can use a tool

## Decision making

The app uses ephemeral key provided by the backend to communicate with Gemini to make decisions

## Program output

The app will use the context of user speech to either do nothing, speak up, or update database. Gemini Live can decide which action to take as a tool use.

+ Speak (Grok Voice API speak the text provided)
+ Launch timer event (a scheduled call to Gemini again in some time notifying the timer ended and ask for decision)
+ Do nothing
+ Save data to database (user preference, note, work session summary). This is done by sending request to backend since the client shouldn't modify database.

## Session end

Tell user how they did in that session, give summary:

+ Time spent
+ Break
+ Attention level

## Tech stack

Gemini
Rust app

## Example usage

00:00

User:
"I want to finish problem 3 in thirty minutes."

Gemini:
create session

Timer:
30:00
05:00

focus = .88
engagement = .86

No event generated.

AI:
silent
11:30

focus = .42
low_focus_duration = 10 sec

AI:
silent
11:45

focus = .31
low_focus_duration = 25 sec

EVENT:
SUSTAINED_LOW_FOCUS
Gemini chooses:
speak(
  "You seem to have slowed down. Did you hit a blocker?"
)
ElevenLabs speaks it.
User:
"Yeah, I'm stuck on the priority queue."
EVENT:
USER_SPOKE
Gemini chooses:
updateGoal(
  "Implement priority queue"
)

setTimer(
  600
)

speak(
  "Let's narrow it down. Spend ten minutes just getting the priority queue working."
)
Session resumes.
15:00

focus = .82

AI:
silent

16:00
Screenshot saw: Youtube shorts.

Gemini decide to intercept

AI: You seems to be off track.

17:00
Screenshot saw: Youtube tutorial on task relevant topic.

AI: do nothing
