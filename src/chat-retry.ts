export function isTemporaryChatError(error: unknown): boolean {
  return /Gemini error (429|500|502|503|504)\b|RESOURCE_EXHAUSTED|UNAVAILABLE|overloaded|high demand|timed? out|timeout|error sending request|Gemini returned no text/i.test(String(error));
}

export async function retryChat<T>(
  message: string,
  send: (message: string) => Promise<T>,
  onDelay: () => void,
  sleep: (ms: number) => Promise<void> = ms => new Promise(resolve => setTimeout(resolve, ms)),
): Promise<T> {
  const delays = [2000, 4000, 8000, 16000, 30000];
  for (let attempt = 0; ; attempt++) {
    try {
      return await send(message);
    } catch (error) {
      if (!isTemporaryChatError(error) || attempt >= delays.length) throw error;
      onDelay();
      const hint = /"retryDelay"\s*:\s*"([\d.]+)s"/.exec(String(error));
      const suggested = hint ? Number(hint[1]) * 1000 : 0;
      await sleep(Math.min(60000, Math.max(delays[attempt], suggested)));
    }
  }
}
