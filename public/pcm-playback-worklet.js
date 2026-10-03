/**
 * Continuous PCM playback — always fills the output from a float32 ring.
 * Underruns are silence (no AudioBufferSourceNode gaps / re-preroll).
 */
class PcmPlaybackProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    // Live often streams faster than realtime; a short ring would drop the start of long
    // replies. 30s is plenty and only costs a few MB.
    this.capacity = Math.max(48000, Math.floor(sampleRate)) * 30;
    this.ring = new Float32Array(this.capacity);
    this.write = 0;
    this.read = 0;
    this.available = 0;
    this.port.onmessage = (event) => {
      const data = event.data || {};
      if (data.type === "reset") {
        this.write = 0;
        this.read = 0;
        this.available = 0;
        return;
      }
      if (data.type === "pcm" && data.samples instanceof Float32Array) {
        const samples = data.samples;
        for (let i = 0; i < samples.length; i += 1) {
          if (this.available >= this.capacity) {
            // Last-resort overflow guard: drop oldest.
            this.read = (this.read + 1) % this.capacity;
            this.available -= 1;
          }
          this.ring[this.write] = samples[i];
          this.write = (this.write + 1) % this.capacity;
          this.available += 1;
        }
      }
    };
  }

  process(_inputs, outputs) {
    const channel = outputs[0] && outputs[0][0];
    if (!channel) return true;
    for (let i = 0; i < channel.length; i += 1) {
      if (this.available > 0) {
        channel[i] = this.ring[this.read];
        this.read = (this.read + 1) % this.capacity;
        this.available -= 1;
      } else {
        channel[i] = 0;
      }
    }
    return true;
  }
}

registerProcessor("pcm-playback", PcmPlaybackProcessor);
