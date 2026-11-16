import test from "node:test";
import assert from "node:assert/strict";
import { cleanTitle } from "./title.js";

const cases = [
  {
    name: "browser and chat app come off the title",
    raw: "Frontend prompt for product redesign - Claude - Google Chrome – BHAVIT",
    hint: { browserProfile: "BHAVIT" },
    want: { title: "Frontend prompt for product redesign", source: "Claude in Chrome" },
  },
  {
    name: "YouTube comes off and the audio flag is kept",
    raw: "Eating at every Tollywood celebrity restaurant for 24 hours - YouTube",
    hint: { app: "Google Chrome", audio: true },
    want: {
      title: "Eating at every Tollywood celebrity restaurant for 24 hours",
      source: "YouTube in Chrome, audio playing",
    },
  },
  {
    name: "a profile name is noise, not a source",
    raw: "Untitled chat — Claude — Profile 1",
    hint: { app: "Google Chrome" },
    want: { title: "Untitled chat", source: "Claude in Chrome" },
  },
  {
    name: "a work app stays, the browser goes",
    raw: "Ticket #42 - Jira - Google Chrome",
    want: { title: "Ticket #42", source: "Jira in Chrome" },
  },
  {
    name: "a site name survives when there is no app segment",
    raw: "Watch The Family Man - Netflix - Safari",
    want: { title: "Watch The Family Man", source: "Netflix in Safari" },
  },
  {
    name: "search counts come off",
    raw: "redesign system - Google Search - Chrome - 3 more pages",
    want: { title: "redesign system", source: "Chrome" },
  },
  {
    name: "a bare title keeps the app as its source",
    raw: "Muse",
    hint: { app: "Freebuff" },
    want: { title: "Muse", source: "Freebuff" },
  },
  {
    name: "an empty title never reaches the screen",
    raw: "   ",
    hint: { app: "Safari" },
    want: { title: "Untitled work", source: "Safari" },
  },
  {
    name: "a title that is only app names still yields a title",
    raw: "Google Chrome",
    want: { title: "Google Chrome", source: "Chrome" },
  },
  {
    name: "Safari removes its witnessed profile",
    raw: "Research notes — Safari — Travail",
    hint: { browserProfile: "Travail" },
    want: { title: "Research notes", source: "Safari" },
  },
  {
    name: "Arc removes its witnessed profile",
    raw: "Planning — Arc — Personal",
    hint: { browserProfile: "Personal" },
    want: { title: "Planning", source: "Arc" },
  },
  {
    name: "Edge removes its witnessed profile",
    raw: "Issue 42 — Edge — Équipe",
    hint: { browserProfile: "Équipe" },
    want: { title: "Issue 42", source: "Edge" },
  },
  {
    name: "Firefox removes its witnessed profile",
    raw: "旅程 — Firefox — 日本語",
    hint: { browserProfile: "日本語" },
    want: { title: "旅程", source: "Firefox" },
  },
];

for (const c of cases) {
  test(`cleaner: ${c.name}`, () => {
    assert.deepEqual(cleanTitle(c.raw, c.hint), c.want);
  });
}

test("cleaner: audio flag is not duplicated", () => {
  const out = cleanTitle("A talk - YouTube - Audio playing", { app: "Chrome", audio: true });
  assert.equal(out.source, "YouTube in Chrome, audio playing");
});
