import assert from 'node:assert/strict';

// Shared by the browser test runner and Codex's browser tab API. No mocked media.
export async function selectMentalistEpisode(tab, baseURL = 'http://localhost:3001') {
  await tab.goto(`${baseURL}/title/series/tmdb%3Aseries%3A5920`);
  await tab.playwright.getByRole('button', { name: 'Open episode 1: Pilot', exact: true }).click();
  await tab.playwright.getByRole('heading', { name: 'Pilot', exact: true }).waitFor({ state: 'visible' });
  await tab.playwright.getByRole('button', { name: 'Choose another playback source', exact: true }).click();
  await tab.playwright.getByRole('list', { name: 'Available playback sources' }).waitFor({ state: 'visible', timeoutMs: 60_000 });
  assert.ok(await tab.playwright.locator('[data-source-option]').count(), 'The episode must have selectable AIOStreams sources');
}

export async function readPlayback(tab) {
  return tab.playwright.evaluate(() => {
    const video = document.querySelector('video');
    if (!video) return null;
    return {
      time: video.currentTime,
      duration: video.duration,
      width: video.videoWidth,
      height: video.videoHeight,
      readyState: video.readyState,
      paused: video.paused,
      error: video.error?.message ?? null,
      source: video.currentSrc,
    };
  });
}

export function assertEpisodePlaying(before, after) {
  assert.equal(after.error, null, 'The media decoder must not report an error');
  assert.ok(after.duration > 40 * 60 && after.duration < 50 * 60,
    `Expected the 45-minute Pilot, not a provider error video (duration ${after.duration})`);
  assert.ok(after.width >= 640 && after.height >= 360, 'Actual episode frames must decode');
  assert.ok(after.readyState >= 2 && !after.paused, 'Video must be playing');
  assert.ok(after.time - before.time >= 3, 'Playback must advance by at least three seconds');
  assert.match(after.source, /(?:blob:|\/v1\/playback\/sessions\/)/, 'Video must use the Reel media route or its HLS MediaSource');
}
