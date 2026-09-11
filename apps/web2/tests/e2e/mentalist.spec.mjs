import { test, expect } from '@playwright/test';
import { selectMentalistEpisode, readPlayback, assertEpisodePlaying } from './mentalist-playback.mjs';

test('The Mentalist S01E01 discovers AIOStreams and plays the actual episode in Reel', async ({ page, baseURL }) => {
  const tab = { goto: url => page.goto(url), playwright: page };
  const discoveryResponse = page.waitForResponse(response => response.url().endsWith('/v1/playback/sources') && response.ok());
  await selectMentalistEpisode(tab, baseURL);
  const discovery = await (await discoveryResponse).json();
  const remoteIndex = discovery.sources.filter(source => source.webReady)
    .findIndex(source => source.source === 'aioStreams');
  expect(remoteIndex, 'The episode must expose a selectable AIOStreams source').toBeGreaterThanOrEqual(0);

  const activationResponse = page.waitForResponse(response => response.url().endsWith('/v1/playback/activate'));
  // A local Jellyfin copy may later appear; this test must still exercise AIOStreams.
  await page.locator('[data-source-option]').nth(remoteIndex).click();
  const activation = await activationResponse;
  const descriptor = await activation.json();
  expect(activation.ok(), descriptor.error?.message ?? 'Playback activation must succeed').toBe(true);
  expect(descriptor.source).toBe('aioStreams');
  expect(descriptor.delivery).toBe('hls');
  expect(descriptor.mediaUrl).toMatch(/^\/v1\/playback\/sessions\//);
  await expect(page.getByRole('button', { name: 'Pause', exact: true })).toBeVisible();

  // Advancing time alone also passes for the provider's two-minute error video.
  // Require the episode's duration, decoded picture, and new decoded frames.
  await expect.poll(async () => (await readPlayback(tab))?.readyState).toBeGreaterThanOrEqual(2);
  const before = await readPlayback(tab);
  await expect.poll(async () => (await readPlayback(tab))?.time - before.time).toBeGreaterThanOrEqual(3);
  assertEpisodePlaying(before, await readPlayback(tab));
  const frames = await page.locator('video').evaluate(video => video.getVideoPlaybackQuality().totalVideoFrames);
  await expect.poll(() => page.locator('video').evaluate(video => video.getVideoPlaybackQuality().totalVideoFrames)).toBeGreaterThan(frames);
  await page.screenshot({ path: test.info().outputPath('mentalist-playing.png') });
  await page.locator('video').click();
});
