export default class Hls {
  static instances = [];
  static Events = Object.fromEntries([
    'MANIFEST_PARSED', 'AUDIO_TRACK_SWITCHED', 'SUBTITLE_TRACK_SWITCH',
    'SUBTITLE_TRACKS_UPDATED', 'ERROR',
  ].map(name => [name, name]));
  static supported = true;
  static ErrorTypes = {
    NETWORK_ERROR: 'networkError',
    MEDIA_ERROR: 'mediaError',
  };
  static isSupported() { return Hls.supported; }
  constructor(config) {
    this.config = config;
    this.handlers = new Map();
    this.audioTracks = [];
    this.subtitleTracks = [];
    this.audioTrack = 0;
    this.subtitleTrack = -1;
    this.subtitleDisplay = true;
    Hls.instances.push(this);
  }
  on(event, handler) {
    const handlers = this.handlers.get(event) ?? [];
    handlers.push(handler);
    this.handlers.set(event, handlers);
  }
  emit(event, data = {}) {
    for (const handler of this.handlers.get(event) ?? []) handler(event, data);
  }
  loadSource(url) { this.url = url; }
  attachMedia(video) { this.video = video; }
  startLoad() { this.startLoadCalls = (this.startLoadCalls ?? 0) + 1; }
  recoverMediaError() { this.recoverMediaErrorCalls = (this.recoverMediaErrorCalls ?? 0) + 1; }
  destroy() { this.handlers.clear(); }
}
