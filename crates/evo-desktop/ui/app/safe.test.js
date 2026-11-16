// A work title is text from another application. It must never be parsed as
// HTML: the classic payload below has to render as those literal characters.
import test from "node:test";
import assert from "node:assert/strict";
import { paintCard, iconSvg } from "./card-view.js";

function node() {
  return {
    textContent: "",
    written: 0,
    get innerHTML() {
      return "";
    },
    set innerHTML(_value) {
      throw new Error("innerHTML must never be written");
    },
    setAttribute(name, value) {
      this[name] = value;
    },
  };
}

const PAYLOAD = '<img src=x onerror=alert(1)>';

test("a hostile work title is rendered as plain text", () => {
  const nodes = { title: node(), when: node(), source: node(), icon: node() };
  const work = { title: PAYLOAD, source: "Chrome", kind: "chat", opened: Date.now() };
  paintCard(nodes, work, () => "Last open just now");
  assert.equal(nodes.title.textContent, PAYLOAD);
  assert.equal(String(nodes.title.innerHTML), "");
});

test("a hostile kind cannot inject markup into the icon", () => {
  const doc = {
    createElementNS: (_ns, tag) => {
      const created = { tag, attributes: {}, children: [], setAttribute(k, v) { this.attributes[k] = v; } };
      created.append = (child) => created.children.push(child);
      return created;
    },
  };
  const svg = iconSvg(doc, `${PAYLOAD}`);
  const use = svg.children[0];
  assert.equal(use.tag, "use");
  assert.equal(use.attributes.href, "#chat", "unknown kinds fall back to the known set");
});

test("the empty card says so without inventing a work", () => {
  const nodes = { title: node(), when: node(), source: node(), icon: node() };
  paintCard(nodes, null, () => "");
  assert.equal(nodes.title.textContent, "Nothing here yet.");
  assert.equal(nodes.when.textContent, "");
});
