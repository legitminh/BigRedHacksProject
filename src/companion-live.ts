/**
 * Copilot / lock-in Live voice client (rebuilt).
 *
 * - Talks only to Waypoint `/v1/companion/live` (wp1 binary PCM).
 * - Desktop owns 1× playback from an unbounded worklet queue.
 * - Opening preroll only — never holds mid-utterance audio as "recovery".
 * - Mic muted while assistant audio is queued/playing.
 * - Screencap always refused (voice-only).
 */

import { invoke } from "@tauri-apps/api/core";

export type CompanionPhase =
  | "idle"
  | "connecting"
  | "listening"
  | "thinking"
  | "speaking";

export type CompanionContext = {
  goals?: string;
  notes?: string;
  modality?: string;
  duration_mins?: number;
  remaining_mins?: number;
  next_step_secs?: number;
  paused?: boolean;
  /** Partial Google Calendar agenda from the desktop (same source as typed Copilot). */
  calendar_summary?: string;
  /** Partial Google Drive listing from the desktop. */
  drive_summary?: string;
};

export type CompanionLiveHandlers = {
  onPhase?: (phase: CompanionPhase) => void;
  onUser?: (text: string, isFinal: boolean) => void;
  onAssistant?: (text: string, isFinal: boolean) => void;
  onError?: (message: string) => void;
  onLevel?: (rms: number) => void;
};

type LiveInfo = { ws_url: string; protocols: string[] };

const AUDIO_PROTOCOL = "wp1";
const AUDIO_KIND_DOWNLINK = 1;
const AUDIO_KIND_UPLINK = 2;
const HEADER_BYTES = 16;
const SCREENCAP_REFUSED = "Live Copilot does not use screen capture.";

/** Opening cushion before the worklet starts draining (Bluetooth-friendly). */
const MIN_PREROLL_SEC = 0.28;
const MAX_PREROLL_SEC = 0.5;
/** Flush a short greeting that never fills the full cushion. */
const PREROLL_IDLE_FLUSH_MS = 90;
/** Keep uplink muted after the worklet drains. */
const POST_PLAYBACK_MUTE_MS = 300;

/**
 * AudioContext created in the mic-click turn. WKWebView leaves a context
 * constructed after `await` suspended, so downlink PCM is queued and never plays.
 */
let primedOutput: AudioContext | null = null;

export function primeCompanionOutput(): void {
  if (primedOutput && primedOutput.state !== "closed") {
    void primedOutput.resume();
    return;
  }
  const ctx = new AudioContext();
  primedOutput = ctx;
  void ctx.resume();
}

function takePrimedOutput(): AudioContext | null {
  const ctx = primedOutput;
  primedOutput = null;
  if (!ctx || ctx.state === "closed") return null;
  return ctx;
}

function audioBufferFrom(data: unknown): ArrayBuffer | null {
  if (data instanceof ArrayBuffer) return data;
  if (ArrayBuffer.isView(data)) {
    const view = data as ArrayBufferView;
    return view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength);
  }
  return null;
}

function audioDebugEnabled(): boolean {
  try {
    return window.localStorage.getItem("wp.audioDebug") === "1";
  } catch {
    return false;
  }
}

function encodePcmFrame(
  kind: number,
  epoch: number,
  sampleRate: number,
  seq: number,
  pcm: ArrayBuffer,
): ArrayBuffer {
  const pcmBytes = new Uint8Array(pcm);
  const out = new ArrayBuffer(HEADER_BYTES + pcmBytes.length);
  const view = new DataView(out);
  view.setUint8(0, 0x57);
  view.setUint8(1, 0x50);
  view.setUint8(2, 1);
  view.setUint8(3, kind);
  view.setUint32(4, epoch >>> 0, false);
  view.setUint32(8, sampleRate >>> 0, false);
  view.setUint32(12, seq >>> 0, false);
  new Uint8Array(out, HEADER_BYTES).set(pcmBytes);
  return out;
}

function decodePcmFrame(buffer: ArrayBuffer): {
  kind: number;
  epoch: number;
  sampleRate: number;
  seq: number;
  pcm: Uint8Array;
} | null {
  if (buffer.byteLength < HEADER_BYTES) return null;
  const view = new DataView(buffer);
  if (view.getUint8(0) !== 0x57 || view.getUint8(1) !== 0x50 || view.getUint8(2) !== 1) {
    return null;
  }
  const kind = view.getUint8(3);
  if (kind !== AUDIO_KIND_DOWNLINK && kind !== AUDIO_KIND_UPLINK) return null;
  return {
    kind,
    epoch: view.getUint32(4, false),
    sampleRate: view.getUint32(8, false),
    seq: view.getUint32(12, false),
    pcm: new Uint8Array(buffer, HEADER_BYTES),
  };
}

/** Linear resample with cross-chunk carry so seams stay continuous. */
function resampleS16leToF32(
  pcm: Uint8Array,
  inputRate: number,
  outputRate: number,
  carry: number | null,
): { samples: Float32Array; last: number | null } {
  const inCount = pcm.length >> 1;
  if (inCount === 0) return { samples: new Float32Array(0), last: carry };
  const view = new DataView(pcm.buffer, pcm.byteOffset, pcm.byteLength);
  const at = (i: number): number => {
    if (i < 0) return carry ?? view.getInt16(0, true) / 32768;
    return view.getInt16(Math.min(inCount - 1, i) * 2, true) / 32768;
  };

  if (inputRate === outputRate) {
    const out = new Float32Array(inCount);
    for (let i = 0; i < inCount; i += 1) out[i] = at(i);
    return { samples: out, last: out[out.length - 1] ?? carry };
  }

  const outCount = Math.max(1, Math.round((inCount * outputRate) / inputRate));
  const out = new Float32Array(outCount);
  const step = inputRate / outputRate;
  let pos = carry == null ? 0 : -step;
  for (let i = 0; i < outCount; i += 1) {
    const i0 = Math.floor(pos);
    const frac = pos - i0;
    out[i] = at(i0) + (at(i0 + 1) - at(i0)) * frac;
    pos += step;
  }
  return { samples: out, last: at(inCount - 1) };
}

export class CompanionLiveSession {
  private socket: WebSocket | null = null;
  private audioCtx: AudioContext | null = null;
  private media: MediaStream | null = null;
  private captureWorklet: AudioWorkletNode | null = null;
  private playbackWorklet: AudioWorkletNode | null = null;

  private phase: CompanionPhase = "idle";
  private epoch = 1;
  private uplinkSeq = 0;
  private downlinkRate = 24_000;
  private useBinaryAudio = true;

  private pending: Float32Array[] = [];
  private pendingSamples = 0;
  private primed = false;
  private prerollSec = MIN_PREROLL_SEC;
  private prerollTimer: number | null = null;

  private speaking = false;
  private oddByte: number | null = null;
  private resampleCarry: number | null = null;
  private queuedSamples = 0;
  private muteUntil = 0;
  private debug = false;

  private handlers: CompanionLiveHandlers;

  constructor(handlers: CompanionLiveHandlers = {}) {
    this.handlers = handlers;
  }

  get active(): boolean {
    return this.phase !== "idle";
  }

  get currentPhase(): CompanionPhase {
    return this.phase;
  }

  private setPhase(next: CompanionPhase) {
    this.phase = next;
    this.handlers.onPhase?.(next);
  }

  private sendJson(payload: Record<string, unknown>) {
    if (this.socket?.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify(payload));
    }
  }

  /** True while assistant audio is audible or still queued. */
  private get playbackBusy(): boolean {
    if (this.muteUntil > 0 && performance.now() < this.muteUntil) return true;
    if (this.queuedSamples > 0) return true;
    if (this.pendingSamples > 0) return true;
    return this.speaking || this.primed;
  }

  private clearPrerollTimer() {
    if (this.prerollTimer == null) return;
    window.clearTimeout(this.prerollTimer);
    this.prerollTimer = null;
  }

  private armPrerollFlush() {
    this.clearPrerollTimer();
    this.prerollTimer = window.setTimeout(() => {
      this.prerollTimer = null;
      if (this.primed || this.pendingSamples === 0) return;
      this.primed = true;
      this.flushPending();
    }, PREROLL_IDLE_FLUSH_MS);
  }

  private stopPlayback() {
    if (this.debug && (this.queuedSamples > 0 || this.pendingSamples > 0)) {
      const rate = this.audioCtx?.sampleRate ?? 48_000;
      console.warn(
        `[live-audio] discard ${((this.queuedSamples + this.pendingSamples) / rate).toFixed(3)}s`,
      );
    }
    this.clearPrerollTimer();
    this.pending = [];
    this.pendingSamples = 0;
    this.primed = false;
    this.speaking = false;
    this.oddByte = null;
    this.resampleCarry = null;
    this.queuedSamples = 0;
    this.muteUntil = 0;
    this.playbackWorklet?.port.postMessage({ type: "reset" });
  }

  private pushToWorklet(samples: Float32Array) {
    if (!this.playbackWorklet || samples.length === 0) return;
    // Do not transfer the buffer. WKWebView's AudioWorklet receives that
    // transfer empty, the processor drops it, and the reply never starts.
    this.playbackWorklet.port.postMessage({ type: "pcm", samples });
  }

  private flushPending() {
    if (!this.primed || !this.playbackWorklet) return;
    this.clearPrerollTimer();
    for (const chunk of this.pending) this.pushToWorklet(chunk);
    this.pending = [];
    this.pendingSamples = 0;
  }

  private updatePreroll() {
    const ctx = this.audioCtx;
    if (!ctx) return;
    const hw = ctx.outputLatency || ctx.baseLatency || 0;
    this.prerollSec = Math.min(MAX_PREROLL_SEC, Math.max(MIN_PREROLL_SEC, hw * 0.8 + 0.22));
  }

  private appendPcm(bytes: Uint8Array) {
    if (bytes.length === 0 || !this.audioCtx) return;
    if (this.audioCtx.state === "suspended") void this.audioCtx.resume();

    let input = bytes;
    if (this.oddByte != null) {
      const merged = new Uint8Array(1 + bytes.length);
      merged[0] = this.oddByte;
      merged.set(bytes, 1);
      input = merged;
      this.oddByte = null;
    }
    if (input.length % 2 === 1) {
      this.oddByte = input[input.length - 1] ?? null;
      input = input.subarray(0, input.length - 1);
    }
    if (input.length === 0) return;

    const rate = this.downlinkRate > 0 ? this.downlinkRate : 24_000;
    const { samples, last } = resampleS16leToF32(
      input,
      rate,
      this.audioCtx.sampleRate,
      this.resampleCarry,
    );
    this.resampleCarry = last;
    if (samples.length === 0) return;

    if (!this.speaking) this.updatePreroll();
    this.speaking = true;
    this.muteUntil = 0;

    // Opening cushion only — never re-gate mid-utterance.
    if (!this.primed) {
      this.pending.push(samples);
      this.pendingSamples += samples.length;
      const target = Math.floor(this.prerollSec * this.audioCtx.sampleRate);
      if (this.pendingSamples >= target) {
        this.primed = true;
        this.flushPending();
      } else {
        this.armPrerollFlush();
      }
      return;
    }
    this.pushToWorklet(samples);
  }

  private enqueueBase64(base64: string, sampleRate: number, messageEpoch: number) {
    if (!this.audioCtx || messageEpoch !== this.epoch) return;
    if (sampleRate > 0) this.downlinkRate = sampleRate;
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
    this.appendPcm(bytes);
  }

  private sendUplink(pcm: ArrayBuffer) {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return;
    if (this.useBinaryAudio) {
      const seq = this.uplinkSeq;
      this.uplinkSeq = (this.uplinkSeq + 1) >>> 0;
      this.socket.send(encodePcmFrame(AUDIO_KIND_UPLINK, this.epoch, 16_000, seq, pcm));
      return;
    }
    const bytes = new Uint8Array(pcm);
    let binary = "";
    const stride = 0x8000;
    for (let i = 0; i < bytes.length; i += stride) {
      binary += String.fromCharCode(...bytes.subarray(i, i + stride));
    }
    this.sendJson({ type: "audio", pcm: btoa(binary) });
  }

  private async ensurePlayback() {
    if (!this.audioCtx || this.playbackWorklet) return;
    await this.audioCtx.audioWorklet.addModule("/pcm-playback-worklet.js");
    this.playbackWorklet = new AudioWorkletNode(this.audioCtx, "pcm-playback");
    this.playbackWorklet.port.onmessage = (event: MessageEvent) => {
      const data = event.data as { type?: string; queued?: number };
      if (typeof data?.queued === "number") this.queuedSamples = data.queued;
      if (data?.type === "underrun") {
        if (this.debug) console.warn(`[live-audio] underrun queued=${this.queuedSamples}`);
        // Only re-arm opening cushion after the reply has fully ended.
        if (!this.speaking && this.queuedSamples === 0) {
          this.primed = false;
          this.muteUntil = performance.now() + POST_PLAYBACK_MUTE_MS;
        }
      }
    };
    this.playbackWorklet.connect(this.audioCtx.destination);
  }

  private async acquireMic() {
    this.media = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
        channelCount: 1,
      },
      video: false,
    });
  }

  private async startMic() {
    if (!this.audioCtx || !this.media) return;
    await this.audioCtx.audioWorklet.addModule("/pcm-worklet.js");
    const source = this.audioCtx.createMediaStreamSource(this.media);
    this.captureWorklet = new AudioWorkletNode(this.audioCtx, "pcm-capture");
    this.captureWorklet.port.onmessage = (event: MessageEvent) => {
      const data = event.data as { pcm?: ArrayBuffer; rms?: number };
      if (typeof data.rms === "number") this.handlers.onLevel?.(data.rms);
      if (!data.pcm) return;
      if (this.playbackBusy) return;
      this.sendUplink(data.pcm);
    };
    source.connect(this.captureWorklet);
  }

  private stopMic() {
    if (this.captureWorklet) {
      this.captureWorklet.disconnect();
      this.captureWorklet = null;
    }
    if (this.media) {
      for (const track of this.media.getTracks()) track.stop();
      this.media = null;
    }
  }

  private handleBinary(buffer: ArrayBuffer) {
    const frame = decodePcmFrame(buffer);
    if (!frame || frame.kind !== AUDIO_KIND_DOWNLINK) return;
    if (frame.epoch !== this.epoch) return;
    if (frame.sampleRate > 0) this.downlinkRate = frame.sampleRate;
    this.appendPcm(frame.pcm);
  }

  private handleMessage(raw: string) {
    let message: Record<string, unknown>;
    try {
      message = JSON.parse(raw) as Record<string, unknown>;
    } catch {
      return;
    }
    const type = message.type;

    if (type === "ready") {
      if (message.audio_protocol === AUDIO_PROTOCOL) this.useBinaryAudio = true;
      return;
    }
    if (type === "status" && typeof message.phase === "string") {
      this.setPhase(message.phase as CompanionPhase);
      return;
    }
    if (type === "user" && typeof message.text === "string") {
      this.handlers.onUser?.(message.text, Boolean(message.final));
      return;
    }
    if (type === "assistant" && typeof message.text === "string") {
      this.handlers.onAssistant?.(message.text, Boolean(message.final));
      return;
    }
    if (type === "audio" && typeof message.pcm === "string") {
      const rate = typeof message.sample_rate === "number" ? message.sample_rate : 24_000;
      const messageEpoch = typeof message.epoch === "number" ? message.epoch : this.epoch;
      this.enqueueBase64(message.pcm, rate, messageEpoch);
      return;
    }
    if (type === "audio_end") {
      if (message.epoch === this.epoch) {
        this.speaking = false;
        if (!this.primed && this.pendingSamples > 0) {
          this.primed = true;
          this.flushPending();
        }
        if (this.queuedSamples === 0 && this.pendingSamples === 0) {
          this.muteUntil = performance.now() + POST_PLAYBACK_MUTE_MS;
        }
      }
      return;
    }
    if (type === "clear_audio" && typeof message.epoch === "number") {
      this.epoch = message.epoch;
      this.uplinkSeq = 0;
      this.stopPlayback();
      return;
    }
    if (type === "screencap_request" && typeof message.id === "string") {
      this.sendJson({
        type: "screencap",
        id: message.id,
        ok: false,
        error: SCREENCAP_REFUSED,
      });
      return;
    }
    if (type === "speak_local" && typeof message.text === "string") {
      // Grok TTS failed — fall back to macOS say (same path as heads-ups).
      const text = message.text.trim();
      if (text) void invoke("voice_speak", { text }).catch(() => {});
      return;
    }
    if (type === "error" && typeof message.message === "string") {
      this.handlers.onError?.(message.message);
      this.end(false);
    }
  }

  async start(context: CompanionContext): Promise<void> {
    if (this.phase !== "idle") return;
    this.debug = audioDebugEnabled();
    const info = await invoke<LiveInfo>("companion_live_info");

    // Output context is resumed in the click (see primeCompanionOutput). Opening
    // the mic first still lets Bluetooth HFP settle before we attach the worklet.
    let micError: unknown = null;
    try {
      await this.acquireMic();
    } catch (err) {
      micError = err;
    }

    this.audioCtx = takePrimedOutput() ?? new AudioContext();
    if (this.audioCtx.state === "suspended") {
      try {
        await this.audioCtx.resume();
      } catch {
        /* playback appendPcm retries resume when PCM arrives */
      }
    }
    await this.ensurePlayback();
    this.updatePreroll();
    if (this.debug) {
      console.info(
        `[live-audio] ctx ${this.audioCtx.sampleRate}Hz out=${this.audioCtx.outputLatency ?? "?"} preroll=${this.prerollSec.toFixed(3)}s`,
      );
    }

    this.setPhase("connecting");
    this.useBinaryAudio = true;
    this.uplinkSeq = 0;
    this.epoch = 1;

    this.socket = new WebSocket(info.ws_url, info.protocols);
    this.socket.binaryType = "arraybuffer";
    await new Promise<void>((resolve, reject) => {
      if (!this.socket) return reject(new Error("No socket"));
      this.socket.onopen = () => {
        if (this.socket) this.socket.binaryType = "arraybuffer";
        resolve();
      };
      this.socket.onerror = () =>
        reject(new Error("Couldn't start live voice. Check your connection and try again."));
    });
    this.socket.onmessage = (event) => {
      const data: unknown = event.data;
      if (typeof data === "string") {
        this.handleMessage(data);
        return;
      }
      if (data instanceof Blob) {
        void data.arrayBuffer().then((buf) => this.handleBinary(buf));
        return;
      }
      const buffer = audioBufferFrom(data);
      if (buffer) this.handleBinary(buffer);
    };
    this.socket.onerror = () => {
      this.handlers.onError?.(
        "Live voice disconnected. Tap Talk to start again, or type a message.",
      );
    };
    this.socket.onclose = () => {
      if (this.phase !== "idle") this.end(false);
    };

    this.sendJson({ type: "start", context, audio_protocol: AUDIO_PROTOCOL });

    try {
      if (micError) throw micError;
      await this.startMic();
    } catch (err) {
      this.handlers.onError?.(
        `Microphone unavailable (${err instanceof Error ? err.message : String(err)}). You can still type.`,
      );
    }
  }

  sendText(text: string) {
    const trimmed = text.trim();
    if (!trimmed || !this.active) return;
    this.sendJson({ type: "text", text: trimmed });
  }

  end(notify = true) {
    if (notify) this.sendJson({ type: "stop" });
    this.stopMic();
    this.stopPlayback();
    if (this.playbackWorklet) {
      try {
        this.playbackWorklet.disconnect();
      } catch {
        /* ignore */
      }
      this.playbackWorklet = null;
    }
    if (this.socket) {
      this.socket.onclose = null;
      try {
        this.socket.close();
      } catch {
        /* ignore */
      }
      this.socket = null;
    }
    if (this.audioCtx) {
      void this.audioCtx.close().catch(() => {});
      this.audioCtx = null;
    }
    void invoke("voice_stop").catch(() => {});
    this.setPhase("idle");
  }
}
