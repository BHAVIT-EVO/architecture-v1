file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:50
    const widths = headers.map((h, i) => Math.max(h.length, ...rows.map((r) => (r[i] ?? '').length)));
                                              ^

RangeError: Maximum call stack size exceeded
    at file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:50:47
    at Array.map (<anonymous>)
    at table (file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:50:28)
    at columnarTable (file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:65:12)
    at file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:96:56
    at Array.map (<anonymous>)
    at formatMarkdown (file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:91:34)
    at file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:101:24
    at Array.map (<anonymous>)
    at formatMarkdown (file:///opt/homebrew/lib/node_modules/rea-agents/node_modules/incur/dist/Formatter.js:91:34)

Node.js v25.8.1
