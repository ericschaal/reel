# ADR 0002: Choose the Android TV implementation after a playback spike

- Status: Proposed
- Date: 2026-09-09

## Context

The Android TV client needs predictable remote focus, a ten-foot interface,
device-aware media playback, track selection, and playback-session integration.
Sharing React code with the web client would reduce some UI duplication, but the
TV experience and player integration are the higher-risk requirements. Expo SDK
57 supports Android TV through `react-native-tvos` and can also target the web,
making a universal client credible enough to validate.

Google recommends Compose for TV for Android TV interfaces and Media3 for
playback and media-session behavior.

## Proposed decision

Run a focused Expo prototype using the selected cinematic movie detail screen.
Validate remote focus, navigation, authenticated playback, media formats, audio
tracks, and subtitles on Android TV, while also checking the web result.

If the prototype succeeds, use one Expo application with platform-specific UI
where needed. If it fails, build the TV client natively in Kotlin with Compose
for TV and Media3/ExoPlayer, and use a separate React web client.

## Consequences

- The implementation choice remains reversible until the spike produces
  evidence.
- Universal code reuse is accepted only where it preserves a genuine TV
  experience.
- The native fallback remains available if focus or playback behavior is weak.
- The disposable browser prototype remains separate from production client code.

## Sources

- <https://developer.android.com/training/tv/get-started/create>
- <https://developer.android.com/training/tv/playback>
- <https://docs.expo.dev/guides/building-for-tv/>
- <https://docs.expo.dev/versions/v57.0.0/>
