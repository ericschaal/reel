import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import test, { afterEach, beforeEach } from "node:test";
import { JSDOM } from "jsdom";
import ts from "typescript";
import { act, createElement } from "react";

const navigationUrl = new URL("../app/keyboard-navigation.tsx", import.meta.url);
registerHooks({
  load(url, context, nextLoad) {
    if (url === navigationUrl.href) {
      return {
        format: "module",
        shortCircuit: true,
        source: ts.transpileModule(readFileSync(navigationUrl, "utf8"), {
          compilerOptions: {
            jsx: ts.JsxEmit.ReactJSX,
            module: ts.ModuleKind.ESNext,
            target: ts.ScriptTarget.ES2022,
          },
        }).outputText,
      };
    }
    return nextLoad(url, context);
  },
});

const dom = new JSDOM('<!doctype html><div id="root"></div>', {
  url: "http://localhost",
});
Object.assign(globalThis, {
  window: dom.window,
  document: dom.window.document,
  Element: dom.window.Element,
  HTMLElement: dom.window.HTMLElement,
  HTMLInputElement: dom.window.HTMLInputElement,
  IS_REACT_ACT_ENVIRONMENT: true,
});
dom.window.HTMLElement.prototype.scrollIntoView = function () {};

const { createRoot } = await import("react-dom/client");
const { KeyboardNavigation } = await import(navigationUrl.href);
const container = document.getElementById("root");
let root;

beforeEach(async () => {
  root = createRoot(container);
  await act(() => root.render(createElement(KeyboardNavigation)));
});

afterEach(async () => {
  document.querySelectorAll("body > :not(#root)").forEach((node) => node.remove());
  delete document.documentElement.dataset.inputModality;
  await act(() => root.unmount());
});

function addButton(label, left, top) {
  const button = document.createElement("button");
  button.textContent = label;
  button.getBoundingClientRect = () => ({
    left,
    top,
    right: left + 100,
    bottom: top + 100,
    width: 100,
    height: 100,
    x: left,
    y: top,
    toJSON() {},
  });
  document.body.append(button);
  return button;
}

function press(target, key, options = {}) {
  target.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      key,
      bubbles: true,
      ...options,
    }),
  );
}

test("arrow keys move focus through a two-dimensional layout", () => {
  const topLeft = addButton("Top left", 0, 0);
  const topRight = addButton("Top right", 140, 0);
  const bottomRight = addButton("Bottom right", 140, 140);
  addButton("Bottom left", 0, 140);

  topLeft.focus();
  press(topLeft, "ArrowRight");
  assert.equal(document.activeElement, topRight);
  press(topRight, "ArrowDown");
  assert.equal(document.activeElement, bottomRight);
});

test("focused rail items scroll to the horizontal center", () => {
  const rail = document.createElement("div");
  rail.dataset.keyboardRail = "true";
  document.body.append(rail);
  const first = addButton("First", 0, 0);
  const second = addButton("Second", 140, 0);
  rail.append(first, second);
  let scrollOptions;
  second.scrollIntoView = (options) => {
    scrollOptions = options;
  };

  first.focus();
  press(first, "ArrowRight");

  assert.equal(document.activeElement, second);
  assert.deepEqual(scrollOptions, {
    behavior: "smooth",
    block: "nearest",
    inline: "center",
  });
});

test("held arrow keys do not queue smooth rail animations", () => {
  const rail = document.createElement("div");
  rail.dataset.keyboardRail = "true";
  document.body.append(rail);
  const first = addButton("First", 0, 0);
  const second = addButton("Second", 140, 0);
  rail.append(first, second);
  let scrollOptions;
  second.scrollIntoView = (options) => {
    scrollOptions = options;
  };

  first.focus();
  press(first, "ArrowRight", { repeat: true });

  assert.equal(document.activeElement, second);
  assert.deepEqual(scrollOptions, {
    behavior: "auto",
    block: "nearest",
    inline: "center",
  });
});

test("horizontal navigation does not escape the active rail", () => {
  const rail = document.createElement("div");
  rail.dataset.keyboardRail = "true";
  document.body.append(rail);
  const railItem = addButton("Rail item", 0, 0);
  rail.append(railItem);
  addButton("Unrelated control", 140, 80);

  railItem.focus();
  press(railItem, "ArrowRight");

  assert.equal(document.activeElement, railItem);
});

test("down from the catalogue menu enters the first content rail", () => {
  const menu = document.createElement("nav");
  menu.dataset.keyboardMenu = "true";
  document.body.append(menu);
  const menuItem = addButton("Movies", 500, 0);
  menu.append(menuItem);

  const rail = document.createElement("div");
  rail.dataset.keyboardRail = "true";
  document.body.append(rail);
  const firstCard = addButton("First card", 0, 180);
  const nearbyAction = addButton("View all", 500, 140);
  rail.append(firstCard);

  menuItem.focus();
  press(menuItem, "ArrowDown");

  assert.equal(document.activeElement, firstCard);
  assert.notEqual(document.activeElement, nearbyAction);
});

test("up into the catalogue menu returns to the current tab", () => {
  const menu = document.createElement("nav");
  menu.dataset.keyboardMenu = "true";
  document.body.append(menu);
  const currentTab = addButton("Discover", 0, 0);
  currentTab.setAttribute("aria-current", "page");
  const nearbyTab = addButton("Movies", 500, 0);
  menu.append(currentTab, nearbyTab);

  const firstRail = document.createElement("div");
  firstRail.dataset.keyboardRail = "true";
  document.body.append(firstRail);
  const card = addButton("Card", 500, 180);
  firstRail.append(card);

  card.focus();
  press(card, "ArrowUp");

  assert.equal(document.activeElement, currentTab);
});

test("spatial focus exposes its direction only during the focus event", () => {
  const first = addButton("First", 0, 0);
  const second = addButton("Second", 140, 0);
  let focusDirection;
  second.addEventListener("focus", () => {
    focusDirection = second.dataset.keyboardFocusDirection;
  });

  first.focus();
  press(first, "ArrowRight");

  assert.equal(focusDirection, "right");
  assert.equal(second.dataset.keyboardFocusDirection, undefined);
});

test("the last-used input modality is exclusive", () => {
  const first = addButton("First", 0, 0);
  addButton("Second", 140, 0);

  first.focus();
  press(first, "ArrowRight");
  assert.equal(document.documentElement.dataset.inputModality, "keyboard");

  document.dispatchEvent(
    new dom.window.MouseEvent("pointermove", {
      bubbles: true,
      clientX: 20,
      clientY: 20,
    }),
  );
  assert.equal(document.documentElement.dataset.inputModality, "pointer");

  press(document.activeElement, "ArrowLeft");
  assert.equal(document.documentElement.dataset.inputModality, "keyboard");
});

test("arrow navigation continues from the card last selected by pointer", () => {
  const first = addButton("First", 0, 0);
  const second = addButton("Second", 140, 0);
  const third = addButton("Third", 280, 0);

  first.focus();
  second.dispatchEvent(
    new dom.window.MouseEvent("pointermove", {
      bubbles: true,
      clientX: 150,
      clientY: 50,
    }),
  );
  press(first, "ArrowRight");
  assert.equal(document.activeElement, third);

  first.dispatchEvent(
    new dom.window.MouseEvent("pointermove", {
      bubbles: true,
      clientX: 10,
      clientY: 50,
    }),
  );
  press(third, "ArrowRight");
  assert.equal(document.activeElement, second);
});

test("stationary pointer events do not pull keyboard navigation toward the cursor", () => {
  const first = addButton("First", 0, 0);
  const second = addButton("Second", 140, 0);
  const third = addButton("Third", 280, 0);

  first.focus();
  second.dispatchEvent(
    new dom.window.MouseEvent("pointermove", {
      bubbles: true,
      clientX: 150,
      clientY: 50,
    }),
  );
  press(first, "ArrowRight");
  assert.equal(document.activeElement, third);

  // The rail can move another card beneath an effectively stationary cursor.
  first.dispatchEvent(
    new dom.window.MouseEvent("pointermove", {
      bubbles: true,
      clientX: 154,
      clientY: 51,
    }),
  );
  assert.equal(document.documentElement.dataset.inputModality, "keyboard");

  press(third, "ArrowLeft");
  assert.equal(document.activeElement, second);
});

test("the first arrow press starts at the current page control", () => {
  const first = addButton("First", 0, 0);
  const current = addButton("Current", 140, 0);
  current.setAttribute("aria-current", "page");

  document.body.focus();
  press(document.body, "ArrowDown");
  assert.equal(document.activeElement, current);
  assert.notEqual(document.activeElement, first);
});

test("native arrow-key controls and managed surfaces keep their own behavior", () => {
  const select = document.createElement("select");
  select.innerHTML = "<option>Season 1</option><option>Season 2</option>";
  select.getBoundingClientRect = () => ({
    left: 0,
    top: 0,
    right: 100,
    bottom: 40,
    width: 100,
    height: 40,
    x: 0,
    y: 0,
    toJSON() {},
  });
  document.body.append(select);
  const next = addButton("Next", 0, 100);
  select.focus();
  press(select, "ArrowDown");
  assert.equal(document.activeElement, select);

  const managed = document.createElement("div");
  managed.dataset.keyboardNavigation = "managed";
  document.body.append(managed);
  next.focus();
  press(next, "ArrowUp");
  assert.equal(document.activeElement, next);
});
