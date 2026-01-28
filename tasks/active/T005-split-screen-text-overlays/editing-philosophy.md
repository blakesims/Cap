# Editing Philosophy: Scaled-Back Style

Reference: [YouTube - Will editing your videos less make you more money?](https://www.youtube.com/watch?v=dWf0AHcwKDk)

## Core Principle

**Less is more.** No fancy animations, no sound effects, no excessive motion. Text appears simply. The human element (body language, facial expressions) stays visible. The system is **repeatable** - same styles every time, no deviation.

## The Three Text Styles

### Style 1: Chapter Marker (Full Screen)
- **Layout:** Full black/dark background, NO camera visible
- **Content:** Just a number or step title, centered
- **Use case:** Marking transitions between sections ("Step 4")
- **Duration:** Brief (2-4 seconds while talking under it)

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│                                                         │
│                       Step 4                            │
│                                                         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Style 2: Split Screen with Bullet Points (PRIMARY)
- **Layout:** Camera on side (40%), text area on other side (60%)
- **Content:** Key points that appear and STAY on screen
- **Animation:** Simple appear or type-on, no motion effects
- **Use case:** 80% of edits - delivering information with human presence

```
┌─────────────────────────────────────────────────────────┐
│                         │                               │
│   • First key point     │                               │
│   • Second key point    │      [Camera Feed]            │
│   • Third key point     │      (You talking)            │
│                         │                               │
└─────────────────────────────────────────────────────────┘
```

### Style 3: PiP with Full Content
- **Layout:** Small camera in corner, content takes full width
- **Content:** Diagrams, more text, complex information
- **Use case:** When you need more space than split-screen allows

```
┌─────────────────────────────────────────────────────────┐
│                                              ┌────────┐ │
│   Detailed content here                      │ [You]  │ │
│   • Point one with explanation               │  PiP   │ │
│   • Point two with explanation               └────────┘ │
│   • Point three with explanation                        │
└─────────────────────────────────────────────────────────┘
```

## Key Constraints

1. **Text ↔ Layout Coupling**
   - If there's text overlay → there's a layout change (split/pip/chapter)
   - If there's split-screen → there's text on the background side
   - They are NEVER independent

2. **No Fiddly Positioning**
   - Text positions are predetermined by the style
   - Bullets stack vertically with consistent spacing
   - No manual X/Y positioning needed

3. **Timing is Relative**
   - Text appearance is relative to segment start
   - "Bullet 2 appears 2 seconds after segment starts"
   - Moving the segment moves all its text together

4. **Preset Styles Only**
   - `title`: Large, bold, centered (for chapter markers)
   - `bullet`: Medium, left-aligned, stacking
   - `numbered`: Same as bullet but with numbers
   - No custom font sizes/colors per item

## What the LLM Workflow Produces

Given a transcript, the LLM identifies:
1. Section breaks → Chapter markers
2. Key points being explained → Split-screen with bullets
3. Complex explanations → PiP with full content

Output is a simple timeline:
```json
{
  "overlays": [
    {
      "type": "chapter",
      "start": 45.0,
      "duration": 3.0,
      "title": "Step 1"
    },
    {
      "type": "split",
      "start": 48.0,
      "end": 120.0,
      "items": [
        { "delay": 0.5, "text": "Key insight one" },
        { "delay": 3.0, "text": "Key insight two" },
        { "delay": 6.0, "text": "Key insight three" }
      ]
    }
  ]
}
```

## Implementation Implications

### Current Problem
- Scene track and Text track are **independent**
- Moving one doesn't move the other
- Easy to break the coupling
- Fiddly to edit

### Proposed Solution: Overlay Track
A new track type that **combines** layout + text:
- One segment = one overlay (chapter/split/pip)
- Dragging the segment moves everything together
- Double-click to edit text content
- Simple duration handles on edges

### Track Behavior
- **Drag segment:** Moves entire overlay (layout + all text)
- **Resize segment:** Extends/shrinks duration, text stays at relative positions
- **Edit content:** Opens simple text editor for items
- **Change type:** Dropdown to switch between chapter/split/pip

### Variables to Expose
| Variable | Editable? | How? |
|----------|-----------|------|
| Segment start/end | Yes | Drag handles |
| Overlay type | Yes | Dropdown |
| Text content | Yes | Click to edit |
| Item delays | Yes | Small handles or number input |
| Background color | Yes | Global setting or per-segment |
| Text color | Yes | Global setting |

### What We DON'T Expose
- Individual text X/Y positions (predetermined by style)
- Font sizes (predetermined by style)
- Animation types (always simple fade/appear)
- Per-item colors (use global)

## Migration Path

1. Keep existing Scene + Text tracks for power users
2. Add new **Overlay Track** as the recommended way
3. Import JSON creates Overlay segments (not raw scene+text)
4. Overlay track internally generates scene+text segments for rendering

This means the rendering pipeline doesn't change - Overlay is a **higher-level abstraction** that generates the low-level segments.
