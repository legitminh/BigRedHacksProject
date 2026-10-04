/**
 * Copilot Live downlink player — unbounded PCM queue, silence on underrun.
 * Never drops samples (that skipped ahead in the reply).
 *
 * Reports {queued, played} so the main thread can mute the mic until drain.
 */
const REPORT_EVERY = 32 * 128; // ~85ms @ 48kHz

/** WKWebView sometimes delivers posted samples as a plain array, not Float32Array. */
function readSamples(value) {
  if (value instanceof Float32Array) return value;
  if (value instanceof ArrayBuffer) return new Float32Array(value);
  if (value && value.buffer instanceof ArrayBuffer && typeof value.length === "number") {
    return new Float32Array(value.buffer, value.byteOffset || 0, value.length);
  }
  if (value && typeof value.length === "number" && value.length > 0) {
    const out = new Float32Array(value.length);
    for (let i = 0; i < value.length; i += 1) out[i] = Number(value[i]) || 0;
    return out;
  }
  return null;
}

class PcmPlaybackProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.offset = 0;
    this.available = 0;
    this.played = 0;
    this.hadAudio = false;
    this.underrunSent = false;
    this.sinceReport = 0;
    this.port.onmessage = (event) => {
      const data = event.data || {};
      if (data.type === "reset") {
        this.queue = [];
        this.offset = 0;
        this.available = 0;
        this.played = 0;
        this.hadAudio = false;
        this.underrunSent = false;
        this.sinceReport = 0;
        return;
      }
      if (data.type === "pcm") {
        const samples = readSamples(data.samples);
        if (!samples || samples.length === 0) return;
        this.queue.push(samples);
        this.available += samples.length;
        this.hadAudio = true;
        this.underrunSent = false;
      }
    };
  }

  report(type) {
    this.port.postMessage({ type, queued: this.available, played: this.played });
  }

  process(_inputs, outputs) {
    const channel = outputs[0] && outputs[0][0];
    if (!channel) return true;

    let filled = 0;
    for (let i = 0; i < channel.length; i += 1) {
      if (this.available > 0 && this.queue.length > 0) {
        const head = this.queue[0];
        channel[i] = head[this.offset];
        this.offset += 1;
        this.available -= 1;
        filled += 1;
        if (this.offset >= head.length) {
          this.queue.shift();
          this.offset = 0;
        }
      } else {
        channel[i] = 0;
      }
    }

    this.played += filled;

    if (filled === 0 && this.hadAudio && !this.underrunSent) {
      this.underrunSent = true;
      this.report("underrun");
      this.sinceReport = 0;
      return true;
    }

    this.sinceReport += channel.length;
    if (this.sinceReport >= REPORT_EVERY) {
      this.sinceReport = 0;
      this.report("level");
    }
    return true;
  }
}

registerProcessor("pcm-playback", PcmPlaybackProcessor);
