Gemini Live vs. what's built. The spec relies on Gemini Live with tool calls, but gemini.rs makes one-shot REST generateContent calls on a timer. Switching to Live means rewriting the main loop around a WebSocket session. Live sessions that include video also have short time limits unless you turn on session resumption or context compression, so a 30-minute session needs reconnect handling.

The focus score doesn't measure focus. The example uses focus = .88, but presage.rs uploads video clips to Presage's cloud and derives focus_ok from your heart rate being between 50 and 110 bpm. That says nothing about attention. Gaze, blink rate, and face presence from the SDK's on-device metrics would be a much better signal, but that means using the C++ SDK instead of the REST upload.

