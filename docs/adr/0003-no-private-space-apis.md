# ADR 0003: No private Space APIs

We do not use private CGS/SkyLight APIs or require SIP changes. macOS Spaces are not programmatically controllable via public APIs. Instead, Evo arranges windows into a collage on the current display using Accessibility and respects visibleFrame. Undo restores frames and unhides apps. If Accessibility is missing, items open normally with a calm explanation.