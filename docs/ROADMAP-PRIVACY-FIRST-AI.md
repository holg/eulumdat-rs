# Roadmap: Privacy-First AI-Augmented Photometric Analysis

> Long-term vision document. Not for immediate implementation, but to be considered for every architectural decision.

## Core Philosophy

**"You get MORE data by respecting privacy MORE."**

The lighting industry is small. Trust is everything. One breach, one rumor of data misuse, and reputation is gone forever. This roadmap ensures we build something that lasts.

## Current State

### What We Have
- Static WASM app served from eulumdat.icu
- Client-side parsing of LDT/IES/ATLA/TM-33
- localStorage for user preferences (theme, language)
- Synthetic sample files (educated guesses, not real measurements)

### What We Lack
- Real-world photometric data for validation
- User preference data on calculation methods
- Industry consensus on edge cases (beam angle definitions, etc.)

---

## Vision: Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 1: User's Browser (100% Private)                     │
│  ─────────────────────────────────────────────────────────  │
│  - All files stay in localStorage                           │
│  - All processing happens client-side                       │
│  - Preferences accumulate locally                           │
│  - NOTHING leaves without explicit consent                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ EXPLICIT OPT-IN
                            │ (shows exactly what will be sent)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 2: Anonymized Preference Aggregation                 │
│  ─────────────────────────────────────────────────────────  │
│  - "User preferred CIE method over IES in 73% of cases"     │
│  - "Typical use case: warehouse lighting"                   │
│  - Statistical deltas, never raw files                      │
│  - k-anonymity: only aggregate when n > threshold           │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ VOLUNTARY CONTRIBUTION
                            │ (credited or anonymous, user decides)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 3: Community Sample Library                          │
│  ─────────────────────────────────────────────────────────  │
│  - Real photometric data from willing contributors          │
│  - Discontinued products (no competitive risk)              │
│  - Academic/lab measurements                                │
│  - Properly licensed, properly credited                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Feature: Side-by-Side Comparison with Swipe Decision

### User Flow

1. User loads multiple files (LDT + IES of same luminaire, or different calculation methods)
2. Side-by-side display: diagrams, values, differences highlighted
3. Tinder-style swipe interface:
   - "Which beam angle looks correct to you?"
   - "Which polar diagram matches your expectation?"
   - Swipe left/right or tap preference
4. Preferences stored LOCALLY
5. After N decisions, optional prompt:
   > "You've made 50 comparison decisions. Would you like to contribute your anonymized preferences to help improve defaults for everyone?"
   >
   > **What will be sent:**
   > - Preferred method A over B: 34 times
   > - Preferred method B over A: 16 times
   > - Experience level: Expert (self-reported)
   > - Use case: Industrial lighting
   >
   > **What will NOT be sent:**
   > - Your files
   > - Company names
   > - Product names
   > - Any identifiable information
   >
   > [Send Anonymously] [Send with Credit] [No Thanks]

### Technical Implementation Notes

```rust
// LocalStorage structure
struct UserPreferences {
    // Private, never leaves browser
    files_analyzed: Vec<FileMetadata>,  // no content, just stats

    // Can be shared anonymously (opt-in)
    method_preferences: HashMap<ComparisonType, (u32, u32)>,  // (a_wins, b_wins)
    experience_level: Option<ExperienceLevel>,
    typical_use_case: Option<UseCase>,

    // Consent tracking
    has_consented_to_sharing: bool,
    consent_timestamp: Option<DateTime>,
    what_was_consented: Option<ConsentScope>,
}
```

---

## Feature: Community Sample Library

### The Problem

Current sample files in `templates/` are synthetic:
- Intensity distributions are "educated guesses"
- Not validated against real measurements
- Limited usefulness for testing edge cases

### The Solution: Transparent Contribution Model

**Public statement on website/README:**

> "Our sample files are synthetic approximations. We want to include real-world photometric data to make the tool more useful for everyone.
>
> If you have LDT/IES files you're allowed to share, consider contributing:
> - Discontinued products (no competitive risk)
> - Generic fixtures (commodity items)
> - Lab measurements (academic use)
> - Old archives (historical preservation)
>
> Contributors can be credited by name/company, or remain fully anonymous. You decide."

### Who Would Contribute?

| Contributor Type | Motivation | Risk Level |
|-----------------|------------|------------|
| Manufacturers (discontinued products) | Free PR, industry goodwill | Low |
| Universities/Labs | Academic credit, open science | Very Low |
| Lighting Designers | Give back, tool improvement | Low |
| Retired Engineers | Legacy, historical preservation | Very Low |
| Enthusiasts | Community participation | Very Low |

### Contribution Workflow

1. User clicks "Contribute Sample Data"
2. Selects file(s) to contribute
3. Chooses:
   - [ ] Credit my name/company
   - [ ] Keep anonymous
   - [ ] License: CC0 / CC-BY / CC-BY-SA
4. Reviews what will be shared:
   - Full file content (with option to strip certain metadata)
   - Optional: description, use case, measurement conditions
5. Submits → Goes to moderation queue
6. After review: added to public sample library with chosen attribution

---

## Data We Want vs. Data We Protect

| Data Type | We Want? | We Protect? | Notes |
|-----------|----------|-------------|-------|
| Raw LDT/IES files | Only donated | ALWAYS | Never harvest, only accept gifts |
| Company names | If credited | If not credited | User's choice |
| Product names | If credited | If not credited | User's choice |
| Method preferences | YES (aggregated) | Individual choices | Only share statistics |
| Experience level | YES (self-reported) | Identity | Helps weight preferences |
| Use cases | YES (categories) | Specific projects | General categories only |
| File statistics | YES (anonymous) | File content | "Average of 37 C-planes" not the values |

---

## Implementation Phases

### Phase 0: Foundation (CURRENT)
- [x] Client-side only processing
- [x] localStorage for preferences
- [x] No server-side data collection
- [x] Open source, auditable

### Phase 1: Local Comparison Tool
- [ ] Side-by-side file comparison view
- [ ] Difference highlighting
- [ ] Local preference storage
- [ ] Export comparison reports (PDF/PNG)

### Phase 2: Swipe Decision Interface
- [ ] Tinder-style A/B preference UI
- [ ] Local preference accumulation
- [ ] Preference-based default suggestions
- [ ] "Why did you choose this?" optional feedback

### Phase 3: Opt-In Contribution
- [ ] Explicit consent dialog (shows exactly what will be sent)
- [ ] Anonymization pipeline
- [ ] k-anonymity thresholds
- [ ] Contribution acknowledgment

### Phase 4: Community Sample Library
- [ ] Contribution workflow
- [ ] Moderation queue
- [ ] Attribution system
- [ ] Public sample browser

### Phase 5: AI-Augmented Suggestions
- [ ] "Users with similar preferences chose X"
- [ ] Anomaly detection ("This value seems unusual")
- [ ] Method recommendations based on use case
- [ ] All powered by AGGREGATED, ANONYMIZED, OPT-IN data

---

## Principles (Non-Negotiable)

1. **Transparency**: Always tell users exactly what we collect and why
2. **Consent**: Nothing leaves the browser without explicit opt-in
3. **Minimization**: Collect only what we need, aggregate whenever possible
4. **Reversibility**: Users can delete their contributions
5. **Auditability**: Open source, users can verify our claims
6. **Benefit First**: User gets value before we ask for anything
7. **Gratitude**: Thank contributors publicly (if they want) or privately

---

## Why This Matters

The lighting industry has a data problem:
- Manufacturers guard their photometric data
- No common repository of validated samples
- Each tool uses different synthetic test files
- Edge cases are never tested against reality

By building trust through radical transparency, we can:
- Create the first community-owned photometric sample library
- Establish industry consensus on calculation methods
- Improve tools for everyone, not just one vendor
- Prove that privacy and useful data can coexist

---

## Technical Considerations for Every Future Change

When implementing ANY new feature, ask:

1. **Does this require data to leave the browser?**
   - If no: good, proceed
   - If yes: is it absolutely necessary? Can we do it client-side?

2. **If data must leave, what's the minimum needed?**
   - Can we aggregate first?
   - Can we anonymize?
   - Can we hash instead of sending raw values?

3. **Is consent explicit and informed?**
   - Does the user see exactly what will be sent?
   - Can they easily say no?
   - Is the default "no sharing"?

4. **Is it auditable?**
   - Can users verify our claims in the source code?
   - Is the data flow documented?

---

## References

- GDPR Article 25: Data Protection by Design and by Default
- Differential Privacy (Dwork et al.)
- k-Anonymity (Sweeney)
- EU Data Governance Act (voluntary data sharing frameworks)

---

*Document created: 2024-12-24*
*Last updated: 2024-12-24*
*Status: Vision/Planning - Not for immediate implementation*
