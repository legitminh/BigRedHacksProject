/**
 * Continuous PCM playback from an unbounded chunk queue.
 * Never drops oldest samples (that skipped ahead in the reply).
 * Underruns are silence; main thread may gently re-preroll after a sustained gap.
 */
class PcmPlaybackProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    /** @type {Float32Array[]} */
    this.queue = [];
    this.offset = 0;
    this.available = 0;
    this.hadAudio = false;
    this.underrunSent = false;
    this.port.onmessage = (event) => {
      const data = event.data || {};
      if (data.type === "reset") {
        this.queue = [];
        this.offset = 0;
        this.available = 0;
        this.hadAudio = false;
        this.underrunSent = false;
        return;
      }
      if (data.type === "pcm" && data.samples instanceof Float32Array) {
        const samples = data.samples;
        if (samples.length === 0) return;
        this.queue.push(samples);
        this.available += samples.length;
        this.hadAudio = true;
        this.underrunSent = false;
      }
    };
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
    // Only signal a full-quantum underrun (boundary hiccups stay silent locally).
    if (filled === 0 && this.hadAudio && !this.underrunSent) {
      this.underrunSent = true;
      this.port.postMessage({ type: "underrun" });
    }
    return true;
  }
}

registerProcessor("pcm-playback", PcmPlaybackProcessor);
