/**
 * Thin Live voice client for Copilot / lock-in.
 * Streams only to Waypoint `/v1/companion/live` — never opens Google sockets.
 *
 * Audio uses the wp1 binary frame protocol (see backend audioProtocol.ts).
 * Downlink plays through a continuous AudioWorklet queue (silence on underrun)
 * so gaps between paced network slices never tear the AudioBufferSource chain.
 *
 * Live never captures the screen — screencap_request is refused immediately.
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
};

export type CompanionLiveHandlers = {
  onPhase?: (phase: CompanionPhase) => void;
  onUser?: (text: string, isFinal: boolean) => void;
  onAssistant?: (text: string, isFinal: boolean) => void;
  onError?: (message: string) => void;
  onLevel?: (rms: number) => void;
};

/** ws_url has no query string; protocols = ["waypoint.live.v1", "bearer.<jwt>"]. */
type LiveInfo = { ws_url: string; protocols: string[] };

const SCREENCAP_REFUSED =
  "Live Copilot does not use screen capture.";

const AUDIO_PROTOCOL = "wp1";
const AUDIO_KIND_DOWNLINK = 1;
const AUDIO_KIND_UPLINK = 2;
const HEADER_BYTES = 16;
/**
 * Jitter cushion before the worklet starts draining.
 * Backend paces ~20ms wp1 slices at 1× realtime — keep this low (~80ms) so speech
 * doesn't feel delayed; brief gaps play as silence rather than a huge buffer.
 */
const PREROLL_SEC = 0.08;
/** After a long underrun gap, rebuild this much cushion before draining again. */
const REPREROLL_SEC = 0.06;
/** Treat an underrun older than this as a real gap worth a gentle re-preroll. */
const UNDERRUN_GAP_MS = 45;
/** Keep mic uplink muted briefly after assistant audio drains (echo / barge-in). */
const POST_PLAYBACK_MUTE_MS = 150;

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
  view.setUint8(0, 0x57); // W
  view.setUint8(1, 0x50); // P
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

/** Resample s16le mono @ inputRate → Float32 at outputRate (linear). */
function resampleS16leToF32(
  pcm: Uint8Array,
  inputRate: number,
  outputRate: number,
): Float32Array {
  const inCount = pcm.length >> 1;
  if (inCount === 0) return new Float32Array(0);
  const view = new DataView(pcm.buffer, pcm.byteOffset, pcm.byteLength);
  if (inputRate === outputRate) {
    const out = new Float32Array(inCount);
    for (let i = 0; i < inCount; i += 1) {
      out[i] = view.getInt16(i * 2, true) / 32768;
    }
    return out;
  }
  const outCount = Math.max(1, Math.round((inCount * outputRate) / inputRate));
  const out = new Float32Array(outCount);
  const ratio = inCount / outCount;
  for (let i = 0; i < outCount; i += 1) {
    const src = i * ratio;
    const i0 = Math.min(inCount - 1, Math.floor(src));
    const i1 = Math.min(inCount - 1, i0 + 1);
    const frac = src - i0;
    const s0 = view.getInt16(i0 * 2, true) / 32768;
    const s1 = view.getInt16(i1 * 2, true) / 32768;
    out[i] = s0 + (s1 - s0) * frac;
  }
  return out;
}

export class CompanionLiveSession {
  private socket: WebSocket | null = null;
  private audioCtx: AudioContext | null = null;
  private media: MediaStream | null = null;
  private captureWorklet: AudioWorkletNode | null = null;
  private playbackWorklet: AudioWorkletNode | null = null;
  private playbackPrimed = false;
  private pendingPlayback: Float32Array[] = [];
  private pendingPlaybackSamples = 0;
  private phase: CompanionPhase = "idle";
  private epoch = 1;
  private oddByte: number | null = null;
  private barged = false;
  private speaking = false;
  private underrunAt = 0;
  private muteUplinkUntil = 0;
  private handlers: CompanionLiveHandlers;
  private useBinaryAudio = true;
  private uplinkSeq = 0;
  private downlinkRate = 24000;

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
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify(payload));
    }
  }

  private stopPlayback() {
    this.pendingPlayback = [];
    this.pendingPlaybackSamples = 0;
    this.playbackPrimed = false;
    this.speaking = false;
    this.underrunAt = 0;
    this.muteUplinkUntil = 0;
    this.oddByte = null;
    // Keep `barged` so in-flight downlink frames stay dropped until clear_audio /
    // listening / audio_end resets it. Clearing here let barge-in leak audio.
    this.playbackWorklet?.port.postMessage({ type: "reset" });
  }

  private pushToPlayback(samples: Float32Array) {
    if (!this.playbackWorklet || samples.length === 0) return;
    // Transfer the underlying buffer when possible to avoid copies.
    this.playbackWorklet.port.postMessage({ type: "pcm", samples }, [samples.buffer]);
  }

  private flushPendingPlayback() {
    if (!this.playbackPrimed || !this.playbackWorklet) return;
    for (const chunk of this.pendingPlayback) {
      this.pushToPlayback(chunk);
    }
    this.pendingPlayback = [];
    this.pendingPlaybackSamples = 0;
  }

  private appendPcmBytes(bytes: Uint8Array) {
    if (bytes.length === 0 || !this.audioCtx || this.barged) return;
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

    const rate = this.downlinkRate > 0 ? this.downlinkRate : 24000;
    const floats = resampleS16leToF32(input, rate, this.audioCtx.sampleRate);
    if (floats.length === 0) return;

    const wasSpeaking = this.speaking;
    this.speaking = true;
    this.muteUplinkUntil = 0;

    // After a real gap (not a 1-quantum hiccup), rebuild a small cushion once.
    if (
      this.playbackPrimed &&
      this.underrunAt > 0 &&
      performance.now() - this.underrunAt >= UNDERRUN_GAP_MS
    ) {
      this.playbackPrimed = false;
    }
    this.underrunAt = 0;

    if (!this.playbackPrimed) {
      this.pendingPlayback.push(floats);
      this.pendingPlaybackSamples += floats.length;
      // First cushion of an utterance uses PREROLL; mid-turn recovery uses REPREROLL.
      const targetSec = wasSpeaking ? REPREROLL_SEC : PREROLL_SEC;
      const target = Math.floor(targetSec * this.audioCtx.sampleRate);
      if (this.pendingPlaybackSamples >= target) {
        this.playbackPrimed = true;
        this.flushPendingPlayback();
      }
      return;
    }
    this.pushToPlayback(floats);
  }

  private enqueuePcmBase64(base64: string, sampleRate: number, messageEpoch: number) {
    if (!this.audioCtx || messageEpoch !== this.epoch) return;
    if (sampleRate > 0) this.downlinkRate = sampleRate;
    const binary = atob(base64);
    const fresh = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) fresh[i] = binary.charCodeAt(i);
    this.appendPcmBytes(fresh);
  }

  private onLevel(rms: number) {
    this.handlers.onLevel?.(rms);
  }

  /** True while assistant audio is queued/playing, or during the post-drain mute hold. */
  private get playbackBusy(): boolean {
    if (this.muteUplinkUntil > 0 && performance.now() < this.muteUplinkUntil) return true;
    return this.speaking || this.playbackPrimed || this.pendingPlaybackSamples > 0;
  }

  private sendUplinkPcm(pcm: ArrayBuffer) {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return;
    if (this.useBinaryAudio) {
      const seq = this.uplinkSeq;
      this.uplinkSeq = (this.uplinkSeq + 1) >>> 0;
      this.socket.send(encodePcmFrame(AUDIO_KIND_UPLINK, this.epoch, 16000, seq, pcm));
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

  private async ensurePlaybackWorklet() {
    if (!this.audioCtx || this.playbackWorklet) return;
    await this.audioCtx.audioWorklet.addModule("/pcm-playback-worklet.js");
    this.playbackWorklet = new AudioWorkletNode(this.audioCtx, "pcm-playback");
    // Underrun → silence in the worklet. Mark the time; only a sustained gap triggers
    // gentle re-preroll on the next packet (see appendPcmBytes). No queue reset/skip.
    this.playbackWorklet.port.onmessage = (event: MessageEvent) => {
      const data = event.data as { type?: string };
      if (data?.type !== "underrun" || this.barged) return;
      this.underrunAt = performance.now();
      if (!this.speaking) {
        this.playbackPrimed = false;
        this.muteUplinkUntil = performance.now() + POST_PLAYBACK_MUTE_MS;
      }
    };
    this.playbackWorklet.connect(this.audioCtx.destination);
  }

  private async startMic() {
    if (!this.audioCtx) return;
    this.media = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
        channelCount: 1,
      },
      video: false,
    });
    await this.audioCtx.audioWorklet.addModule("/pcm-worklet.js");
    const source = this.audioCtx.createMediaStreamSource(this.media);
    this.captureWorklet = new AudioWorkletNode(this.audioCtx, "pcm-capture");
    this.captureWorklet.port.onmessage = (event: MessageEvent) => {
      const data = event.data as { pcm?: ArrayBuffer; rms?: number };
      if (typeof data.rms === "number") this.onLevel(data.rms);
      if (!data.pcm) return;
      // Hard-mute uplink while assistant audio is queued/playing (and briefly after).
      // Speaker echo was reaching Gemini VAD and cutting the reply short.
      if (this.playbackBusy || this.barged) return;
      this.sendUplinkPcm(data.pcm);
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
    this.appendPcmBytes(frame.pcm);
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
      if (message.phase === "listening") {
        this.barged = false;
        // Do not clear playbackPrimed here. Server sends listening immediately after
        // audio_end while the worklet may still be draining; clearing primed early
        // made playbackBusy false and unmuted the mic into speaker echo.
        // Underrun (and clear_audio / stopPlayback) re-preroll the next turn.
      }
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
      const rate =
        typeof message.sample_rate === "number" ? message.sample_rate : 24000;
      const messageEpoch =
        typeof message.epoch === "number" ? message.epoch : this.epoch;
      this.enqueuePcmBase64(message.pcm, rate, messageEpoch);
      return;
    }
    if (type === "audio_end") {
      if (message.epoch === this.epoch) {
        this.barged = false;
        this.speaking = false;
        // Drain any leftover preroll if the utterance was shorter than PREROLL_SEC.
        if (!this.playbackPrimed && this.pendingPlaybackSamples > 0) {
          this.playbackPrimed = true;
          this.flushPendingPlayback();
        }
      }
      return;
    }
    if (type === "clear_audio" && typeof message.epoch === "number") {
      this.epoch = message.epoch;
      this.uplinkSeq = 0;
      this.barged = false;
      this.stopPlayback();
      return;
    }
    if (type === "screencap_request" && typeof message.id === "string") {
      // Live never captures the screen — refuse immediately (no permission prompt, no invoke).
      this.sendJson({
        type: "screencap",
        id: message.id,
        ok: false,
        error: SCREENCAP_REFUSED,
      });
      return;
    }
    if (type === "error" && typeof message.message === "string") {
      this.handlers.onError?.(message.message);
      this.end(false);
    }
  }

  async start(context: CompanionContext): Promise<void> {
    if (this.phase !== "idle") return;
    const info = await invoke<LiveInfo>("companion_live_info");
    // Prefer the hardware rate so we don't fight the OS resampler on top of ours.
    this.audioCtx = new AudioContext();
    if (this.audioCtx.state === "suspended") await this.audioCtx.resume();
    await this.ensurePlaybackWorklet();
    this.setPhase("connecting");
    this.useBinaryAudio = true;
    this.uplinkSeq = 0;
    this.epoch = 1;

    // JWT rides in Sec-WebSocket-Protocol (bearer.<jwt>), never the URL. The API selects
    // "waypoint.live.v1" and does not echo the token. Don't log ws_url/protocols.
    this.socket = new WebSocket(info.ws_url, info.protocols);
    this.socket.binaryType = "arraybuffer";
    await new Promise<void>((resolve, reject) => {
      if (!this.socket) return reject(new Error("No socket"));
      this.socket.onopen = () => resolve();
      this.socket.onerror = () =>
        reject(new Error("Couldn't start live voice. Check your connection and try again."));
    });
    this.socket.onmessage = (event) => {
      if (typeof event.data === "string") this.handleMessage(event.data);
      else if (event.data instanceof ArrayBuffer) this.handleBinary(event.data);
      else if (event.data instanceof Blob) {
        void event.data.arrayBuffer().then((buf) => this.handleBinary(buf));
      }
    };
    this.socket.onerror = () => {
      this.handlers.onError?.(
        "Live voice disconnected. Tap Talk to start again, or type a message.",
      );
    };
    this.socket.onclose = () => {
      if (this.phase !== "idle") this.end(false);
    };
    this.sendJson({
      type: "start",
      context,
      audio_protocol: AUDIO_PROTOCOL,
    });
    try {
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
    this.barged = false;
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
