# Design Decisions Matrix

## Layout & Space

| Question | Decision | Notes |
|----------|----------|-------|
| Text too long / line wrapping | **User handles it** | Text is editable, user can shorten if needed. Solo app, no edge case handling. |
| Vertical spacing algorithm | **TBD** | Need recommendation |
| Text positioning specifics | **Configurable in UI** | Should be adjustable without editing JSON |
| Max items per overlay | **No limit** | User responsibility to keep reasonable |

## Transitions & Gaps

| Question | Decision | Notes |
|----------|----------|-------|
| What shows between overlays? | **Default PiP view** | Investigate: does this make sense architecturally? |
| What happens when overlay ends? | **Need recommendation** | Fade out? Hard cut? |
| Back-to-back overlays | **Need recommendation** | Smooth or hard transition? |

## Camera & Recording

| Question | Decision | Notes |
|----------|----------|-------|
| No camera recordings | **Not supported** | Always assume camera exists |
| Camera position in split | **Fixed: RIGHT side (40%)** | Never configurable |
| Text position in split | **Fixed: LEFT side (60%)** | Never configurable |
| PiP position | **Fixed: Bottom-right** | Standard PiP behavior |

## The Three Styles - Clarified

| Style | Camera | Text Area | Use Case |
|-------|--------|-----------|----------|
| **Default/Gap** | PiP bottom-right | None | Between overlays |
| **Split** | Right 40% | Left 60% with bullets | Information delivery |
| **Chapter** | Hidden | Full screen, centered title | Section transitions |
| ~~PiP with content~~ | ~~Corner~~ | ~~Full width~~ | **Descoped for now** |

## Chapter Markers

| Question | Decision | Notes |
|----------|----------|-------|
| Audio during chapter? | **Need recommendation** | User talking over? Silent pause? |
| Chapter duration | **Configurable default** | e.g., 3 seconds, adjustable |
| Chapter background | **Use Cap's background setting** | Same as split-screen background |

## Background

| Question | Decision | Notes |
|----------|----------|-------|
| What is background? | **Cap's existing background system** | Wallpapers, colors, gradients - already exists |
| Per-overlay or global? | **Global** | Use Cap's background setting |

## Track Coexistence

| Question | Decision | Notes |
|----------|----------|-------|
| Overlay vs Scene+Text tracks | **Overlay goes on top** | Takes precedence when active |
| Both visible in UI? | **TBD** | Recommendation needed |
| Migration of old projects | **Ignore** | Not a concern |

## LLM Integration

| Question | Decision | Notes |
|----------|----------|-------|
| Where JSON generated? | **External CLI (`kb edit`)** | Not Cap's concern |
| Error recovery | **Must be easy to fix in UI** | Critical UX requirement |

## Animation

| Question | Decision | Notes |
|----------|----------|-------|
| Fade duration | **In code/config** | Not UI-configurable, but clean code |
| Easing function | **Ease-in-out** | Standard, clean in code |
| Configurable? | **Via code/config file** | Not runtime UI |

## Aspect Ratio

| Question | Decision | Notes |
|----------|----------|-------|
| Vertical video (9:16) | **Defer** | Will test with real workflow |
| Different export sizes | **Defer** | Test empirically |

---

## Finalized Decisions (from T006 Planning)

### 1. Vertical Spacing Algorithm
**Decision:** Fixed spacing (Y += 0.12 between items), start from Y = 0.25

### 2. What Happens When Overlay Ends?
**Decision:** Text slides out left + fades (300ms), then transition to next state

### 3. Back-to-Back Overlays
**Decision:** Allow tiny gaps, no auto-merge. Each overlay transitions independently.

### 4. Audio During Chapter Markers
**Decision:** Audio continues (talking over the card)

### 5. Both Tracks Visible in UI?
**Decision:** User manages - no auto-hide of Scene/Text tracks when Overlay exists

---

## Fixed Constraints Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                     SPLIT SCREEN LAYOUT                        │
│                                                                 │
│   ┌─────────────────────────┬─────────────────────────────┐    │
│   │                         │                             │    │
│   │   TEXT AREA (60%)       │   CAMERA (40%)              │    │
│   │   • Bullets here        │   Always right side         │    │
│   │   • Left-aligned        │   Cropped/fitted            │    │
│   │                         │                             │    │
│   └─────────────────────────┴─────────────────────────────┘    │
│                                                                 │
│   Background: Cap's background setting (color/gradient/image)  │
│   Text color: Configurable (default white)                     │
│   Position: Configurable in UI (margins, vertical start)       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     CHAPTER MARKER                              │
│                                                                 │
│   ┌─────────────────────────────────────────────────────────┐  │
│   │                                                         │  │
│   │                      Step 4                             │  │
│   │                                                         │  │
│   │            (Centered title, large text)                 │  │
│   │                                                         │  │
│   └─────────────────────────────────────────────────────────┘  │
│                                                                 │
│   Camera: HIDDEN                                                │
│   Audio: Continues (talking over)                               │
│   Duration: Configurable default (e.g., 3s)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     DEFAULT / GAP VIEW                          │
│                                                                 │
│   ┌─────────────────────────────────────────────────────────┐  │
│   │                                                         │  │
│   │                                                         │  │
│   │                    [Screen Recording]                   │  │
│   │                                                         │  │
│   │                                         ┌─────────┐     │  │
│   │                                         │ Camera  │     │  │
│   │                                         │  PiP    │     │  │
│   │                                         └─────────┘     │  │
│   └─────────────────────────────────────────────────────────┘  │
│                                                                 │
│   This is what shows when NO overlay is active                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Descoped (Not in V1)

- PiP-with-full-content style (small camera corner + wide text)
- Vertical video support
- Per-item animation configuration
- Camera position choice (always right)
- Migration of old projects
