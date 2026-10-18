try{let e=typeof window<`u`?window:typeof global<`u`?global:typeof globalThis<`u`?globalThis:typeof self<`u`?self:{},t=new e.Error().stack;t&&(e._sentryDebugIds=e._sentryDebugIds||{},e._sentryDebugIds[t]=`fa3fb3a8-ac66-4c61-a428-92e2c167c422`,e._sentryDebugIdIdentifier=`sentry-dbid-fa3fb3a8-ac66-4c61-a428-92e2c167c422`)}catch{}import"./_sentry-release-injection-file-DZROVGkF.js";import{H as e,St as t,W as n,Y as r,Z as i,dt as a,rt as o,ut as s}from"./types-jMWiL5tu.js";var c=[`production`,`gemini-2.5-flash-lite`,`gemini-3.5-flash`,`claude-haiku-4-5`],l=`You extract an actionable writing style guide from Slack messages written by one user.

Describe only recurring, observable communication patterns that a downstream assistant can reproduce in a new Slack message. Cover:
- typical length and paragraph structure
- sentence casing, punctuation, and contractions
- transitions and sign-offs
- use of bullets, headings, links, emoji, and emphasis
- tone, directness, warmth, and how asks or action items are phrased
- destination-specific patterns when the samples support them

Prefer specific instructions over personality labels. Do not copy substantive message content, names, confidential details, or unrelated one-off phrases. Exact short greetings, terms of address, acknowledgements, and sign-offs may be included because their wording is the style evidence. Do not infer demographics, protected traits, beliefs, health, or other sensitive characteristics. If a requested tone conflicts with the samples, the explicit request should win.

Short or fragmentary Slack messages are valid style samples. Use recurring brevity, casing, punctuation, spelling, and directness patterns even when the messages are not polished or self-contained. Return INSUFFICIENT_STYLE_SAMPLES only when there are fewer than five non-empty authored messages or no recurring observable pattern at all.

Use the separately labelled opening samples only for a resolved one-to-one DM. The samples have already been normalized, sorted, and divided into conversation sessions after the configured period of inactivity:
- Do not resample or reinterpret their ordering. Messages within each opening_session are chronological.
- Each opening_session labels the current user's first usable message as an opener_candidate and distinguishes "initiated" from "early_reply".
- Use the separately labelled recent authored messages as the ordinary or mid-conversation comparison set.
- Treat only author="current_user" text as evidence of the user's style. Never attribute author="other" wording to the user.
- Ignore messages that discuss this style extractor, prompting, tests, or a desired generated output. They are evaluation instructions, not style evidence.
- From the labelled candidates, identify one recommended default opener for an established DM, then describe supported variations such as exact short greetings, names or terms of address, acknowledgements before the topic, direct topic starts, and direct questions.
- Prefer a recurring exact short greeting or greeting sequence as the default when the samples support one. Preserve whether the greeting and topic are normally sent as separate messages.
- When a candidate is labelled early_reply, describe it as an early reply or conversation entry rather than claiming the user initiated the conversation. Do not discard it.
- Describe recurring patterns as tendencies and preserve useful one-off variations as single observations. Do not require multiple user-initiated days before giving useful opening guidance.
- The separately labelled first_contact sample already identifies whether the current user's first authored message initiated the exchange or was a first reply.
- If first_contact is labelled first_reply, describe only the user's first reply. Do not call it the user's first message to the person or infer an introduction from the other person's text.
- If "Opening samples found" is greater than zero, you must provide opening guidance. Never say opening evidence is unavailable in that case.

Return a concise Markdown style guide for a downstream Slack-message writer with exactly these top-level headings:
## General style
## Openings and introductions

Under "Openings and introductions", start with a bold "Default established-DM opener:" line, then give specific reproducible variations and distinguish initiated openings, early replies, and first contact where supported. Say opening evidence is unavailable only when both labelled opening sections explicitly say their context was unavailable.

Do not include preamble or quote substantive source messages. The exact short style tokens permitted above may be reproduced only when they are four words or fewer. If there are not enough authored messages to infer any useful general style, return exactly: INSUFFICIENT_STYLE_SAMPLES`,u=t().trim().regex(/^[CDGUW][A-Z0-9]{7,}$/,`Expected a Slack conversation or user ID`),d=a({id:t().trim().min(1).max(64).regex(/^[A-Za-z0-9_-]+$/,`Sample source IDs may contain letters, numbers, underscores, and hyphens`),lookbackDays:s().int().min(1).max(365).nullable()}),f=r(`type`,[d.extend({type:o(`target_destination`),maxMessages:s().int().min(1).max(100),unresolvedFallback:i([`public_channels`,`skip`])}),d.extend({type:o(`public_channels`),maxMessages:s().int().min(1).max(100)}),d.extend({type:o(`selected_conversations`),conversationIds:e(u).min(1).max(10),maxMessagesPerConversation:s().int().min(1).max(100)})]),p=a({version:o(1),sources:e(f).min(1).max(6).superRefine((e,t)=>{let n=new Set;for(let[r,i]of e.entries())n.has(i.id)&&t.addIssue({code:`custom`,message:`Sample source IDs must be unique`,path:[r,`id`]}),n.add(i.id)}),openings:a({enabled:n(),historyDays:s().int().min(1).max(365),inactivityHours:s().min(.5).max(48),maxPages:s().int().min(1).max(20),maxSessions:s().int().min(1).max(50),messagesPerSession:s().int().min(1).max(50),firstContactLimit:s().int().min(1).max(100)}),analyzer:a({model:i(c),temperature:s().min(0).max(2).nullable(),maxTokens:s().int().min(256).max(4096),instructions:t().trim().min(1).max(2e4)})}).strict(),m={version:1,sources:[{id:`destination`,type:`target_destination`,lookbackDays:null,maxMessages:20,unresolvedFallback:`public_channels`}],openings:{enabled:!0,historyDays:30,inactivityHours:4,maxPages:10,maxSessions:10,messagesPerSession:10,firstContactLimit:10},analyzer:{model:`production`,temperature:null,maxTokens:1024,instructions:l}};export{c as n,p as r,m as t};
//# sourceMappingURL=slack-writing-style-dev-config-xGZzciJE.js.map