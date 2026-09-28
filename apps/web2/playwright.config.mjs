import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 180_000,
  expect: { timeout: 60_000 },
  workers: 1,
  retries: 0,
  use: {
    baseURL: process.env.REEL_WEB_URL ?? 'http://localhost:3001',
    // Traces can capture scoped media tokens, so keep them disabled.
    trace: 'off',
    screenshot: 'only-on-failure',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'], launchOptions: { args: ['--autoplay-policy=no-user-gesture-required'] } } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
});
