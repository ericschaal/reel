import assert from "node:assert/strict";
import test from "node:test";
import { createDebouncedPublisher } from "../app/debounced-publisher.ts";

test("ambient updates publish only the latest selection after it settles", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const published = [];
  const scheduler = createDebouncedPublisher(120, (value) => {
    published.push(value);
  });

  scheduler.schedule("first");
  t.mock.timers.tick(60);
  scheduler.schedule("second");
  t.mock.timers.tick(119);
  assert.deepEqual(published, []);

  t.mock.timers.tick(1);
  assert.deepEqual(published, ["second"]);
  scheduler.dispose();
});

test("disposing an ambient update prevents a late publication", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const published = [];
  const scheduler = createDebouncedPublisher(120, (value) => {
    published.push(value);
  });

  scheduler.schedule("selection");
  scheduler.dispose();
  t.mock.timers.tick(120);

  assert.deepEqual(published, []);
});
