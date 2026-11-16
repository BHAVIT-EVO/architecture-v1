import test from "node:test";
import assert from "node:assert/strict";
import { workType } from "./type.js";

const cases = [
  ["a Claude chat is a book", { source: "Claude in Chrome" }, "book"],
  ["a browser page is a book", { source: "Chrome" }, "book"],
  ["a PDF in Preview is paper", { source: "Preview" }, "paper"],
  ["a YouTube video is paper", { source: "YouTube in Chrome, audio playing" }, "paper"],
  ["a Google Doc is paper", { source: "Google Docs in Chrome" }, "paper"],
  ["Finder is a folder", { source: "Finder" }, "folder"],
  ["a code project is a folder", { source: "Cursor" }, "folder"],
  ["a terminal is a folder", { source: "Terminal" }, "folder"],
  ["notes are a notebook", { source: "Notes" }, "notebook"],
  ["an unknown source is a notebook", { source: "Unknown source" }, "notebook"],
  ["explicit kind wins", { kind: "paper", source: "Claude in Chrome" }, "paper"],
];

for (const [name, input, want] of cases) {
  test(`mapping: ${name}`, () => assert.equal(workType(input), want));
}
