/**
 * Thin Gemini Live client for study sessions.
 * Talks only to Waypoint API `/v1/companion/live` — never to Google / Gemini directly.
 * Patterns adapted from legitminh/gemini_live_demo (assets/index.html).
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

function bytesToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const stride = 0x8000;
  for (let i = 0; i < bytes.length; i += stride) {
    binary += String.fromCharCode(...bytes.subarray(i, i + stride));
  }
  return btoa(binary);
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
  private leftover = new Uint8Array(0);
  private bargeHits = 0;
  private barged = false;
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
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify(payload));
    }
  }

  private stopPlayback() {
    for (const source of this.sources) {
      try {
        source.stop();
      } catch {
        /* already stopped */
      }
    }
    this.sources = [];
    this.playhead = 0;
    this.leftover = new Uint8Array(0);
    this.barged = false;
    this.bargeHits = 0;
  }

  private enqueuePcm(base64: string, sampleRate: number, messageEpoch: number) {
    if (!this.audioCtx || messageEpoch !== this.epoch) return;
    const binary = atob(base64);
    const fresh = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) fresh[i] = binary.charCodeAt(i);
    const combined = new Uint8Array(this.leftover.length + fresh.length);
    combined.set(this.leftover, 0);
    combined.set(fresh, this.leftover.length);
    const samples = combined.length >> 1;
    this.leftover = combined.slice(samples * 2);
    if (samples === 0) return;
    const floats = new Float32Array(samples);
    const view = new DataView(combined.buffer, combined.byteOffset, samples * 2);
    for (let i = 0; i < samples; i += 1) floats[i] = view.getInt16(i * 2, true) / 32768;
    const rate = sampleRate > 0 ? sampleRate : 24000;
    const buffer = this.audioCtx.createBuffer(1, samples, rate);
    buffer.copyToChannel(floats, 0);
    const source = this.audioCtx.createBufferSource();
    source.buffer = buffer;
    source.connect(this.audioCtx.destination);
    const now = this.audioCtx.currentTime;
    if (this.playhead < now + 0.02) this.playhead = now + 0.02;
    source.start(this.playhead);
    this.playhead += buffer.duration;
    this.sources.push(source);
    source.onended = () => {
      this.sources = this.sources.filter((item) => item !== source);
    };
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
        this.sendJson({ type: "audio", pcm: bytesToBase64(data.pcm) });
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

  private handleMessage(raw: string) {
    let message: Record<string, unknown>;
    try {
      message = JSON.parse(raw) as Record<string, unknown>;
    } catch {
      return;
    }
    const type = message.type;
    if (type === "ready") return;
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
      this.enqueuePcm(message.pcm, rate, messageEpoch);
      return;
    }
    if (type === "audio_end") {
      if (message.epoch === this.epoch) this.barged = false;
      return;
    }
    if (type === "clear_audio" && typeof message.epoch === "number") {
      this.epoch = message.epoch;
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
    this.audioCtx = new AudioContext();
    if (this.audioCtx.state === "suspended") await this.audioCtx.resume();
    this.setPhase("connecting");

    const url = `${info.ws_url}?access_token=${encodeURIComponent(info.access_token)}`;
    this.socket = new WebSocket(url);
    await new Promise<void>((resolve, reject) => {
      if (!this.socket) return reject(new Error("No socket"));
      this.socket.onopen = () => resolve();
      this.socket.onerror = () =>
        reject(new Error("Could not open the companion Live socket"));
    });
    this.socket.onmessage = (event) => this.handleMessage(String(event.data));
    this.socket.onerror = () => {
      this.handlers.onError?.("The companion voice connection failed.");
    };
    this.socket.onclose = () => {
      if (this.phase !== "idle") this.end(false);
    };
    this.sendJson({ type: "start", context });
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
