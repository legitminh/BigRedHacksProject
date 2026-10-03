/**
 * Thin Live voice client for Copilot / lock-in.
 * Streams only to Waypoint `/v1/companion/live` — never opens Google sockets.
 *
 * Audio uses the wp1 binary frame protocol (see backend audioProtocol.ts)
 * with a client jitter buffer so playback stays continuous.
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

type LiveInfo = { ws_url: string; access_token: string };

const AUDIO_PROTOCOL = "wp1";
const AUDIO_KIND_DOWNLINK = 1;
const AUDIO_KIND_UPLINK = 2;
const HEADER_BYTES = 16;
/** Wait this long before starting playback (smooths network jitter). */
const PREROLL_SEC = 0.12;
/** Prefer scheduling chunks of at least this duration. */
const MIN_CHUNK_SEC = 0.06;
/** If the playhead falls behind, rebuild with this lead. */
const RECOVER_LEAD_SEC = 0.08;

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

export class CompanionLiveSession {
  private socket: WebSocket | null = null;
  private audioCtx: AudioContext | null = null;
  private media: MediaStream | null = null;
  private worklet: AudioWorkletNode | null = null;
  private phase: CompanionPhase = "idle";
  private epoch = 1;
  private playhead = 0;
  private sources: AudioBufferSourceNode[] = [];
  private queueRaw = new Uint8Array(0);
  private oddByte: number | null = null;
  private bargeHits = 0;
  private barged = false;
  private handlers: CompanionLiveHandlers;
  private useBinaryAudio = true;
  private uplinkSeq = 0;
  private downlinkRate = 24000;
  private startedPlayback = false;
  private pendingSamples = 0;
  private pumpTimer: number | undefined;

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
    if (this.pumpTimer != null) {
      window.clearInterval(this.pumpTimer);
      this.pumpTimer = undefined;
    }
    for (const source of this.sources) {
      try {
        source.stop();
      } catch {
        /* already stopped */
      }
    }
    this.sources = [];
    this.playhead = 0;
    this.queueRaw = new Uint8Array(0);
    this.oddByte = null;
    this.pendingSamples = 0;
    this.startedPlayback = false;
    this.barged = false;
    this.bargeHits = 0;
  }

  private appendPcmBytes(bytes: Uint8Array) {
    if (bytes.length === 0) return;
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
    this.queueRaw = this.concatUint8(this.queueRaw, input);
    this.pendingSamples = this.queueRaw.length >> 1;
    this.ensurePump();
  }

  private concatUint8(a: Uint8Array, b: Uint8Array): Uint8Array {
    if (a.length === 0) return b.slice();
    if (b.length === 0) return a;
    const out = new Uint8Array(a.length + b.length);
    out.set(a, 0);
    out.set(b, a.length);
    return out;
  }

  private ensurePump() {
    if (this.pumpTimer != null) return;
    this.pumpTimer = window.setInterval(() => this.pumpPlayback(), 20);
  }

  private takeSamples(count: number): Float32Array | null {
    const need = count * 2;
    if (this.queueRaw.length < need) return null;
    const slice = this.queueRaw.subarray(0, need);
    this.queueRaw = this.queueRaw.slice(need);
    this.pendingSamples = this.queueRaw.length >> 1;
    const floats = new Float32Array(count);
    const view = new DataView(slice.buffer, slice.byteOffset, slice.byteLength);
    for (let i = 0; i < count; i += 1) {
      floats[i] = view.getInt16(i * 2, true) / 32768;
    }
    return floats;
  }

  private pumpPlayback() {
    if (!this.audioCtx || this.barged) return;
    const rate = this.downlinkRate > 0 ? this.downlinkRate : 24000;
    const pendingSec = this.pendingSamples / rate;

    if (!this.startedPlayback) {
      if (pendingSec < PREROLL_SEC) return;
      this.startedPlayback = true;
      const now = this.audioCtx.currentTime;
      this.playhead = now + RECOVER_LEAD_SEC;
    }

    const now = this.audioCtx.currentTime;
    // Heal underruns instead of scheduling in the past (main source of chop).
    if (this.playhead < now + 0.02) {
      this.playhead = now + RECOVER_LEAD_SEC;
    }

    // Keep ~250ms scheduled ahead when possible.
    const minSamples = Math.floor(MIN_CHUNK_SEC * rate);
    while (this.playhead - now < 0.25) {
      if (this.pendingSamples < minSamples) break;
      const want = Math.min(this.pendingSamples, Math.floor(0.12 * rate));
      const samples = this.takeSamples(want);
      if (!samples) break;
      const buffer = this.audioCtx.createBuffer(1, samples.length, rate);
      buffer.copyToChannel(samples, 0);
      const source = this.audioCtx.createBufferSource();
      source.buffer = buffer;
      source.connect(this.audioCtx.destination);
      source.start(this.playhead);
      this.playhead += buffer.duration;
      this.sources.push(source);
      source.onended = () => {
        this.sources = this.sources.filter((item) => item !== source);
      };
    }

    if (this.pendingSamples === 0 && this.sources.length === 0 && this.startedPlayback) {
      // Idle — allow next utterance to re-preroll.
      this.startedPlayback = false;
    }
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
    const playing =
      this.playhead > (this.audioCtx ? this.audioCtx.currentTime + 0.05 : 0);
    if (playing && rms > 0.12) this.bargeHits += 1;
    else this.bargeHits = 0;
    if (playing && this.bargeHits > 5 && !this.barged) {
      this.barged = true;
      this.sendJson({ type: "barge" });
      this.stopPlayback();
      this.setPhase("listening");
    }
  }

  private sendUplinkPcm(pcm: ArrayBuffer) {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return;
    if (this.useBinaryAudio) {
      const seq = this.uplinkSeq;
      this.uplinkSeq = (this.uplinkSeq + 1) >>> 0;
      this.socket.send(encodePcmFrame(AUDIO_KIND_UPLINK, this.epoch, 16000, seq, pcm));
      return;
    }
    // Fallback JSON base64
    const bytes = new Uint8Array(pcm);
    let binary = "";
    const stride = 0x8000;
    for (let i = 0; i < bytes.length; i += stride) {
      binary += String.fromCharCode(...bytes.subarray(i, i + stride));
    }
    this.sendJson({ type: "audio", pcm: btoa(binary) });
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
    this.worklet = new AudioWorkletNode(this.audioCtx, "pcm-capture");
    this.worklet.port.onmessage = (event: MessageEvent) => {
      const data = event.data as { pcm?: ArrayBuffer; rms?: number };
      if (typeof data.rms === "number") this.handlers.onLevel?.(data.rms);
      if (data.pcm) {
        if (typeof data.rms === "number") this.onLevel(data.rms);
        this.sendUplinkPcm(data.pcm);
      }
    };
    source.connect(this.worklet);
  }

  private stopMic() {
    if (this.worklet) {
      this.worklet.disconnect();
      this.worklet = null;
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
      if (message.phase === "listening") this.barged = false;
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
        // Flush any remainder through the pump.
        this.ensurePump();
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
      void this.handleScreencapRequest(message.id);
      return;
    }
    if (type === "error" && typeof message.message === "string") {
      this.handlers.onError?.(message.message);
      this.end(false);
    }
  }

  private async handleScreencapRequest(id: string) {
    this.setPhase("thinking");
    try {
      const jpeg_base64 = await invoke<string>("companion_grab_screencap");
      this.sendJson({
        type: "screencap",
        id,
        ok: true,
        jpeg_base64,
      });
    } catch (err) {
      this.sendJson({
        type: "screencap",
        id,
        ok: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async start(context: CompanionContext): Promise<void> {
    if (this.phase !== "idle") return;
    const info = await invoke<LiveInfo>("companion_live_info");
    this.audioCtx = new AudioContext({ sampleRate: 48000 });
    if (this.audioCtx.state === "suspended") await this.audioCtx.resume();
    this.setPhase("connecting");
    this.useBinaryAudio = true;
    this.uplinkSeq = 0;
    this.epoch = 1;

    const url = `${info.ws_url}?access_token=${encodeURIComponent(info.access_token)}`;
    this.socket = new WebSocket(url);
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
    this.stopPlayback();
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
