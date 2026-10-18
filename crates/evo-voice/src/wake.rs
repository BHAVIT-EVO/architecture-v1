//! Wake-word spotting: "Hey Evo" detection on transcribed text.
//!
//! This is the streaming-whisper keyword-spot fallback from the GOAL.
//! The production path will be a custom-trained openWakeWord ONNX model
//! (negligible CPU, always-on); this module covers the gap: transcribe
//! audio chunks and recognize the wake phrase. The driver owns command
//! capture: the wake phrase itself is never a command.
//!
//! The wake phrase is the product name, not a hardcoded command — what
//! follows it is resolved by meaning via [`crate::resolve_trigger`], never
//! by fixed phrases.

/// The wake phrase, normalized.
const WAKE_PHRASE: &str = "hey evo";

/// Accepted second-word variants for the wake phrase, primary first.
///
/// Whisper does not always spell the product name the same way on live
/// mic audio: the acoustic path's captures have transcribed it as
/// "Hey, Ego." and "Hey, Evo." A near-homophone in the *name* position is
/// forgiving on purpose; the leading "hey" still has to be there, so
/// ordinary speech ("the ego is fine") never matches. Kept next to
/// [`WAKE_PHRASE`], whose second word must stay in this table (a test
/// enforces it).
const WAKE_NAME_VARIANTS: [&str; 3] = ["evo", "ego", "eva"];

/// Maximum sanitized transcript sent through the voice pipeline.
///
/// This bound is shared by wake scanning, live captions, the final command,
/// and the socket boundary. Whisper output is feedback, not an unbounded
/// model response.
pub const MAX_TRANSCRIPT_BYTES: usize = 4096;

/// Whisper phantom outputs: whole-transcript matches that are dropped.
///
/// These are pre-training memorizations and loop artifacts whisper emits
/// over non-speech (breath, room tone, a passing click) when a capture
/// still passes the energy gate. Live evidence: hallucination-prone
/// segments produced "I bet I'm going to be making a kitsnakaki." and
/// "Look the fuck?" out of noise, alongside the classic YouTube-trained
/// pair the mic transcribed after near-silent captures ("Thank you.",
/// "Thanks for watching").
///
/// Matching is case-insensitive and punctuation-insensitive AFTER
/// normalization, and only when the phantom is the WHOLE transcript —
/// never a substring — so a real command containing these words
/// ("thank you notes from the meeting") survives untouched.
const PHANTOM_TRANSCRIPTS: [&str; 10] = [
    "thank you",
    "thanks",
    "thanks for watching",
    "thank you for watching",
    "you",
    "subtitles by",
    "subtitle by",
    "subtitles",
    "subscribe",
    "please subscribe",
];

/// Result of scanning a transcript for the wake phrase.
#[derive(Debug, PartialEq)]
pub enum WakeResult {
    /// No wake phrase found; keep listening.
    Absent,
    /// Wake phrase found; the normalized suffix is available for callers that
    /// need to remove a repeated wake from a completed capture. The driver
    /// must never dispatch this suffix from the wake scan itself: it starts a
    /// fresh command capture and only its final utterance may be dispatched.
    Detected {
        /// The command text after the wake phrase, trimmed.
        command: String,
    },
}

/// Scans `transcript` for the wake phrase.
///
/// Normalizes case and punctuation before matching. Matches the two whole
/// words "hey evo" anywhere (a mid-utterance wake is still a valid trigger —
/// the user may have started speaking before the mic caught it).
///
/// Returns the command portion (after the wake phrase) so the caller can
/// feed it straight to [`crate::resolve_trigger`].
pub fn detect_wake(transcript: &str) -> WakeResult {
    // Keep Whisper's non-speech annotations out of both the wake match and
    // the command suffix.  Callers normally sanitize before reaching here,
    // but this public boundary is also used directly by probes and tests.
    let normalized = normalize(&sanitize_transcript(transcript));
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let wake_words: Vec<&str> = WAKE_PHRASE.split_whitespace().collect();
    let Some(pos) = words.windows(wake_words.len()).position(|candidate| {
        candidate.len() == wake_words.len()
            && candidate[0] == wake_words[0]
            && WAKE_NAME_VARIANTS.contains(&candidate[1])
    }) else {
        return WakeResult::Absent;
    };
    let command = sanitize_transcript(&words[pos + wake_words.len()..].join(" "));
    WakeResult::Detected { command }
}

/// Removes Whisper's non-speech annotation segments and normalizes the
/// whitespace left behind.
///
/// Whisper commonly returns bracketed placeholders for an empty or inaudible
/// window (`[BLANK_AUDIO]`, `[ Inaudible ]`, `[NOISE]`). They are model
/// metadata, not spoken words: allowing them through would put them in live
/// captions and, after an acoustic wake, could turn silence into a real
/// trigger. Bracketed segments are all treated as metadata at this boundary;
/// a spoken command must arrive as ordinary transcript text.
pub fn sanitize_transcript(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut cursor = 0;
    while cursor < text.len() {
        let Some(open_rel) = text[cursor..].find('[') else {
            cleaned.push_str(&text[cursor..]);
            break;
        };
        let open = cursor + open_rel;
        cleaned.push_str(&text[cursor..open]);
        let Some(close_rel) = text[open + 1..].find(']') else {
            cleaned.push_str(&text[open..]);
            break;
        };
        let close = open + 1 + close_rel;
        // Preserve the boundary in "open[BLANK_AUDIO]notes". Bracketed
        // Whisper annotations, including noisy partial markers, are never
        // spoken command text.
        cleaned.push(' ');
        cursor = close + 1;
    }
    let mut normalized = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() > MAX_TRANSCRIPT_BYTES {
        let mut end = MAX_TRANSCRIPT_BYTES;
        while !normalized.is_char_boundary(end) {
            end -= 1;
        }
        normalized.truncate(end);
        normalized = normalized.trim_end().to_string();
    }
    // A removed annotation can leave punctuation ("[ Inaudible ].").
    // Noise-only text is not a spoken command either.
    if is_non_speech_placeholder(&normalized)
        || is_noise_only(&normalized)
        || is_phantom_transcript(&normalized)
        || !normalized.chars().any(char::is_alphanumeric)
    {
        String::new()
    } else {
        normalized
    }
}

fn is_non_speech_placeholder(text: &str) -> bool {
    let normalized = text
        .chars()
        .map(|character| match character {
            '_' | '-' => ' ',
            character => character.to_ascii_lowercase(),
        })
        .collect::<String>();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    matches!(
        normalized.as_str(),
        "blank audio"
            | "inaudible"
            | "inaudible speech"
            | "no speech"
            | "silence"
            | "noise"
            | "background noise"
    )
}

fn is_noise_only(text: &str) -> bool {
    let normalized = normalize(text);
    let words: Vec<_> = normalized.split_whitespace().collect();
    !words.is_empty()
        && words.iter().all(|word| {
            matches!(
                *word,
                "um" | "uh"
                    | "er"
                    | "erm"
                    | "hmm"
                    | "hm"
                    | "mm"
                    | "mhm"
                    | "background"
                    | "conversation"
                    | "noise"
            )
        })
}

/// True when the whole (normalized) transcript is a known whisper
/// phantom ([`PHANTOM_TRANSCRIPTS`], listed there) or a single-token
/// repetition loop ("the the the", "okay okay okay"). Exact
/// whole-transcript matching: a longer utterance that merely contains a
/// phantom phrase is real speech and is never dropped.
fn is_phantom_transcript(text: &str) -> bool {
    let normalized = normalize(text);
    if normalized.is_empty() {
        // Empty text is handled by the caller's alphanumeric check.
        return false;
    }
    if PHANTOM_TRANSCRIPTS.contains(&normalized.as_str()) {
        return true;
    }
    // A pure loop is one token repeated at least three times and nothing
    // else. Repetition inside a longer utterance can be real speech
    // ("no no no wait"), and two-token repeats are common in commands
    // ("very very"), so both survive.
    let words: Vec<&str> = normalized.split_whitespace().collect();
    words.len() >= 3 && words.iter().all(|word| *word == words[0])
}

/// Lowercases, collapses whitespace, strips punctuation except spaces.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wake_at_start() {
        let r = detect_wake("Hey Evo, take me back to the evo stuff");
        assert_eq!(
            r,
            WakeResult::Detected {
                command: "take me back to the evo stuff".to_string()
            }
        );
    }

    #[test]
    fn detects_wake_mid_utterance() {
        let r = detect_wake("um hey evo open my writing");
        assert_eq!(
            r,
            WakeResult::Detected {
                command: "open my writing".to_string()
            }
        );
    }

    #[test]
    fn absent_when_no_wake() {
        assert_eq!(detect_wake("take me back to work"), WakeResult::Absent);
        assert_eq!(detect_wake(""), WakeResult::Absent);
    }

    #[test]
    fn wake_with_no_command_yields_empty() {
        let r = detect_wake("hey evo");
        assert_eq!(
            r,
            WakeResult::Detected {
                command: String::new()
            }
        );
        assert_eq!(
            detect_wake("hey evo silence"),
            WakeResult::Detected {
                command: String::new()
            }
        );
    }

    #[test]
    fn punctuation_does_not_break_detection() {
        let r = detect_wake("Hey, Evo! restore my session.");
        assert_eq!(
            r,
            WakeResult::Detected {
                command: "restore my session".to_string()
            }
        );
    }

    #[test]
    fn sanitizes_whisper_silence_placeholders() {
        assert_eq!(
            sanitize_transcript("[BLANK_AUDIO] [ Inaudible ] open my writing"),
            "open my writing"
        );
        assert_eq!(sanitize_transcript("[ inaudible ]"), "");
        assert_eq!(
            sanitize_transcript("Please [NOISE] keep this [BLANK_AUDIO] phrase"),
            "Please keep this phrase"
        );
        assert_eq!(sanitize_transcript("[ Inaudible ]."), "");
        assert_eq!(sanitize_transcript("open[BLANK_AUDIO]notes"), "open notes");
        assert_eq!(sanitize_transcript("silence"), "");
        assert_eq!(
            detect_wake("[BLANK_AUDIO] Hey Evo [ Inaudible ]"),
            WakeResult::Detected {
                command: String::new()
            }
        );
    }

    #[test]
    fn wake_match_requires_whole_words_and_transcripts_are_bounded() {
        assert_eq!(detect_wake("they evolved a plan"), WakeResult::Absent);
        let long = format!("open {}", "x".repeat(MAX_TRANSCRIPT_BYTES * 2));
        assert!(sanitize_transcript(&long).len() <= MAX_TRANSCRIPT_BYTES);
    }

    #[test]
    fn wake_variant_table_covers_the_primary_phrase() {
        let parts: Vec<&str> = WAKE_PHRASE.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        assert!(
            WAKE_NAME_VARIANTS.contains(&parts[1]),
            "WAKE_PHRASE's name word must stay in WAKE_NAME_VARIANTS"
        );
    }

    #[test]
    fn fuzzy_wake_variants_strip_only_the_phrase() {
        // The live acoustic captures transcribed the wake as "Hey, Ego."
        // and "Hey, Evo."; the command after the phrase must survive.
        let cases = [
            ("hey ego open my writing", "open my writing"),
            (
                "hey eva, take me back to the evo stuff",
                "take me back to the evo stuff",
            ),
            ("Hey Evo. open it", "open it"),
            ("a hey evo open it", "open it"),
            ("... hey evo open it", "open it"),
            ("hey ego", ""),
        ];
        for (text, expected) in cases {
            assert_eq!(
                detect_wake(text),
                WakeResult::Detected {
                    command: expected.to_string()
                },
                "{text:?}"
            );
        }
        // "ego"/"eva" alone are ordinary words; only "hey <name>" wakes.
        assert_eq!(detect_wake("the ego is fine"), WakeResult::Absent);
        assert_eq!(detect_wake("they evolved a plan"), WakeResult::Absent);
    }

    #[test]
    fn phantom_transcripts_are_dropped_but_real_commands_survive() {
        for text in [
            "Thank you.",
            "thank you",
            "Thanks for watching!",
            "thank you for watching",
            "You",
            "subtitles by",
            "Subscribe",
            "the the the",
        ] {
            assert_eq!(
                sanitize_transcript(text),
                "",
                "{text:?} is a whisper phantom, not a command"
            );
        }
        // Exact whole-transcript matches only: these contain phantom words
        // but are real speech.
        assert_eq!(
            sanitize_transcript("thank you notes from the meeting"),
            "thank you notes from the meeting"
        );
        assert_eq!(
            sanitize_transcript("subtitles by the editor please"),
            "subtitles by the editor please"
        );
        assert_eq!(
            sanitize_transcript("you're right, open it"),
            "you're right, open it"
        );
    }

    #[test]
    fn phantom_outputs_are_rejected() {
        for text in [
            "Thank you",
            "thank you.",
            "THANK YOU!",
            "Thanks",
            "thanks.",
            "Thanks for watching!",
            "thank you for watching",
            "You",
            "you.",
            "subtitles by",
            "Subtitles by.",
            "subtitle by",
            "subtitles",
            "Subscribe",
            "subscribe.",
            "please subscribe",
            "Please, subscribe!",
        ] {
            assert_eq!(
                sanitize_transcript(text),
                "",
                "{text:?} is a whisper phantom, not a command"
            );
        }
    }

    #[test]
    fn real_commands_containing_phantom_words_survive() {
        for text in [
            "thank you notes from the meeting",
            "subscribe to the newsletter draft",
            "you should open the pitch deck",
        ] {
            assert_eq!(sanitize_transcript(text), text, "{text:?} is real speech");
        }
    }

    #[test]
    fn repeated_single_token_loops_are_rejected() {
        // Pure single-token loops (one word three or more times, nothing
        // else) are decode artifacts.
        assert_eq!(sanitize_transcript("the the the"), "");
        assert_eq!(sanitize_transcript("Okay, okay, okay."), "");
        // Decision: only pure repeats of one token are dropped. A repeat
        // inside a longer utterance can be real speech, so "no no no wait"
        // (and any trailing word) survives, and two-token repeats are kept
        // too ("very very") — dropping them risks real commands.
        assert_eq!(sanitize_transcript("no no no wait"), "no no no wait");
        assert_eq!(
            sanitize_transcript("yes yes yes please"),
            "yes yes yes please"
        );
        assert_eq!(sanitize_transcript("the the"), "the the");
    }

    #[test]
    fn noise_only_transcripts_are_empty() {
        for text in [
            "um",
            "[NOISE]",
            "background noise",
            "background conversation",
            "um, background conversation",
        ] {
            assert_eq!(sanitize_transcript(text), "", "{text:?} is not a command");
        }
    }
}
