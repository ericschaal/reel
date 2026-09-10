import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, test } from 'node:test';
import { JSDOM } from 'jsdom';
import ts from 'typescript';
import { act, createElement } from 'react';
import Hls from './fixtures/hls.mjs';

// Exercise the real TSX component using the existing Node test runner.
const playerDirectory = fileURLToPath(new URL('../app/title/[kind]/[id]/', import.meta.url));
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === 'hls.js') {
      return { url: new URL('./fixtures/hls.mjs', import.meta.url).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url.startsWith('file:') && fileURLToPath(url).startsWith(playerDirectory) && /\.tsx?$/.test(url)) {
      return { format: 'module', shortCircuit: true, source: ts.transpileModule(
        readFileSync(new URL(url), 'utf8'),
        { compilerOptions: { jsx: ts.JsxEmit.ReactJSX, module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } },
      ).outputText };
    }
    return nextLoad(url, context);
  },
});
const dom = new JSDOM('<!doctype html><div id="root"></div>', { url: 'http://localhost' });
Object.assign(globalThis, {
  window: dom.window, document: dom.window.document,
  HTMLMediaElement: dom.window.HTMLMediaElement,
  IS_REACT_ACT_ENVIRONMENT: true,
});
const { createRoot } = await import('react-dom/client');
const { ReelVideoPlayer } = await import('../app/title/[kind]/[id]/video-player.tsx');
const container = document.getElementById('root');
let root;
let nativeHls;
let initialTracks;
let mediaState;
const track = (index, label, language) => ({ index, label, language, codec: 'subrip', isDefault: false, isForced: false });
const descriptor = {
  sessionId: 'initial', delivery: 'hls', mediaUrl: '/initial', durationSeconds: 600,
  audioTracks: [track(1, 'Original audio', 'eng'), track(2, 'Alternate audio', 'fra')],
  subtitleTracks: [track(3, 'English', 'eng'), track(4, 'French', 'fra')],
  selectedAudioIndex: 1, selectedSubtitleIndex: -1,
};
function state(video) {
  if (!mediaState.has(video)) {
    const textTracks = Object.assign(new dom.window.EventTarget(), {
      tracks: initialTracks.map(track => ({ ...track })),
      [Symbol.iterator]() { return this.tracks[Symbol.iterator](); },
    });
    mediaState.set(video, { paused: true, textTracks, playCalls: 0 });
  }
  return mediaState.get(video);
}
Object.defineProperties(dom.window.HTMLMediaElement.prototype, {
  paused: { configurable: true, get() { return state(this).paused; } },
  duration: { configurable: true, get() { return 600; } },
  readyState: { configurable: true, get() { return 2; } },
  textTracks: { configurable: true, get() { return state(this).textTracks; } },
  canPlayType: { configurable: true, value() { return nativeHls ? 'probably' : ''; } },
  play: { configurable: true, value() {
    state(this).playCalls++;
    state(this).paused = false;
    this.dispatchEvent(new dom.window.Event('play'));
    return Promise.resolve();
  } },
  pause: { configurable: true, value() {
    state(this).paused = true;
    this.dispatchEvent(new dom.window.Event('pause'));
  } },
  // Paused media need not produce another frame callback.
  requestVideoFrameCallback: { configurable: true, value() { return 1; } },
  cancelVideoFrameCallback: { configurable: true, value() {} },
  // The browser's load algorithm resets speed and permits autoplay again.
  load: { configurable: true, value() {
    state(this).paused = true;
    this.playbackRate = this.defaultPlaybackRate;
    this.dispatchEvent(new dom.window.Event('ratechange'));
  } },
});
beforeEach(() => {
  nativeHls = true;
  initialTracks = [
    { label: 'English', language: 'eng', mode: 'disabled' },
    { label: 'French', language: 'fra', mode: 'disabled' },
  ];
  mediaState = new WeakMap();
  Hls.instances = [];
  root = createRoot(container);
});
afterEach(async () => { await act(() => root.unmount()); });
async function mount(overrides = {}, onSelectTracks = async (_position, selection) => ({
  ...descriptor, ...overrides, mediaUrl: '/replacement', selectedAudioIndex: selection.audioStreamIndex,
  selectedSubtitleIndex: selection.subtitleStreamIndex,
})) {
  await act(async () => root.render(createElement(ReelVideoPlayer, {
    media: { title: 'Test movie' }, playback: { status: 'ready', descriptor: { ...descriptor, ...overrides } },
    onBack() {}, onSelectTracks,
  })));
  return container.querySelector('video');
}
async function click(label) {
  const button = [...container.querySelectorAll('button')].find(button =>
    button.getAttribute('aria-label') === label || button.textContent.trim() === label);
  assert.ok(button, `Button ${label} exists`);
  await act(async () => button.click());
}
async function emit(video, name) { await act(() => video.dispatchEvent(new dom.window.Event(name))); }
async function canPlay(video) {
  await emit(video, 'loadedmetadata');
  await emit(video, 'loadeddata');
  await emit(video, 'canplay');
  if (video.autoplay && video.paused) await act(() => video.play());
}
async function changeAudio() {
  await click('Audio and subtitles');
  await click('Alternate audiofra');
}

test('native HLS selects the requested subtitle even when track order differs', async () => {
  initialTracks.reverse();
  const video = await mount({ selectedSubtitleIndex: 3 });
  assert.deepEqual([...video.textTracks].map(track => track.mode), ['disabled', 'showing']);
});

test('native subtitle Off disables all tracks, including late arrivals', async () => {
  const video = await mount();
  const late = { label: 'Commentary', language: 'eng', mode: 'showing' };
  video.textTracks.tracks.push(late);
  await act(() => video.textTracks.dispatchEvent(new dom.window.Event('addtrack')));
  assert.ok([...video.textTracks].every(track => track.mode === 'disabled'));
});

test('hls.js selects the requested track when rendition tracks become available', async () => {
  nativeHls = false;
  await mount({ selectedSubtitleIndex: 4 });
  const hls = Hls.instances[0];
  await act(() => hls.emit(Hls.Events.MANIFEST_PARSED));
  await act(async () => {
    hls.subtitleTracks = [{ name: 'English', lang: 'eng' }, { name: 'French', lang: 'fra' }];
    hls.emit(Hls.Events.SUBTITLE_TRACKS_UPDATED);
    // hls.js applies its default after notifying SUBTITLE_TRACKS_UPDATED listeners.
    hls.subtitleTrack = 0;
  });
  assert.equal(hls.subtitleTrack, 1);
  assert.equal(hls.subtitleDisplay, true);
});

for (const engine of ['native HLS', 'hls.js']) {
  test(`${engine}: track changes preserve pause state and permit another switch`, async () => {
    nativeHls = engine === 'native HLS';
    let switches = 0;
    const video = await mount({}, async (_position, selection) => ({
      ...descriptor, mediaUrl: `/replacement-${++switches}`,
      selectedAudioIndex: selection.audioStreamIndex, selectedSubtitleIndex: selection.subtitleStreamIndex,
    }));
    await canPlay(video);
    await act(() => video.pause());
    video.currentTime = 120;
    await changeAudio();
    await canPlay(video);
    assert.equal(video.paused, true);
    assert.equal(video.currentTime, 120);
    assert.equal(container.querySelector('[aria-label="Switching track"]'), null);
    assert.equal(container.querySelector('video'), video);
    await click('Audio and subtitles');
    await click('Original audioeng');
    await canPlay(video);
    assert.equal(switches, 2);
    assert.equal(video.paused, true);
  });

  test(`${engine}: track changes resume playing media at the selected speed`, async () => {
    nativeHls = engine === 'native HLS';
    const video = await mount();
    await canPlay(video);
    await click('Playback speed');
    await click('1.5×');
    assert.equal(video.playbackRate, 1.5);
    video.currentTime = 120;
    await changeAudio();
    await canPlay(video);
    assert.equal(video.paused, false);
    assert.equal(video.currentTime, 120);
    assert.equal(video.playbackRate, 1.5);
    assert.equal(container.querySelector('[aria-label="Playback speed"]').textContent, '1.5×');
  });
}

test('external rate changes update the speed control', async () => {
  const video = await mount();
  await act(async () => {
    video.playbackRate = 2;
    video.dispatchEvent(new dom.window.Event('ratechange'));
  });
  assert.equal(container.querySelector('[aria-label="Playback speed"]').textContent, '2×');
});

for (const selectedSubtitleIndex of [-1, null]) {
  test(`hls.js keeps subtitles off for selection ${selectedSubtitleIndex}`, async () => {
    nativeHls = false;
    await mount({ selectedSubtitleIndex });
    const hls = Hls.instances[0];
    await act(async () => {
      hls.subtitleTracks = [{ name: 'English', lang: 'eng' }];
      hls.emit(Hls.Events.SUBTITLE_TRACKS_UPDATED);
      hls.subtitleTrack = 0;
    });
    assert.equal(hls.subtitleTrack, -1);
    assert.equal(hls.subtitleDisplay, false);
  });
}

test('subtitle matching distinguishes two tracks in the same language', async () => {
  initialTracks = [
    { label: 'English commentary', language: 'eng', mode: 'showing' },
    { label: 'English', language: 'eng', mode: 'disabled' },
  ];
  const video = await mount({ selectedSubtitleIndex: 3 });
  assert.deepEqual([...video.textTracks].map(track => track.mode), ['disabled', 'showing']);
});

test('subtitle matching does not select an unrelated or ambiguous track', async () => {
  initialTracks = [
    { label: 'English', language: 'eng', mode: 'showing' },
    { label: 'English', language: 'eng', mode: 'showing' },
  ];
  const video = await mount({ selectedSubtitleIndex: 3 });
  assert.ok([...video.textTracks].every(track => track.mode === 'disabled'));
});

test('failed track activation restores the original stream and playing state', async () => {
  const video = await mount({}, async () => { throw new Error('Track unavailable'); });
  await canPlay(video);
  video.currentTime = 120;
  await changeAudio();
  assert.equal(video.getAttribute('src'), '/initial');
  assert.equal(video.currentTime, 120);
  assert.equal(video.paused, false);
  assert.equal(container.querySelector('[role="alert"]').textContent.trim(), 'Track unavailable');
});

test('leaving while activation is pending does not resume the detached video', async () => {
  let rejectActivation;
  const video = await mount({}, () => new Promise((_resolve, reject) => { rejectActivation = reject; }));
  await canPlay(video);
  await changeAudio();
  await act(() => root.render(null));
  const playCalls = state(video).playCalls;
  await act(async () => rejectActivation(new Error('Aborted')));
  assert.equal(state(video).playCalls, playCalls);
});
