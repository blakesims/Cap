# T006 UX Improvements Plan

## Objective
Add sidebar editing for overlay segments to match the UX pattern of other track types (text, mask, zoom).

## Current State
- Overlays can only be edited via double-click modal (OverlayEditor.tsx)
- Other track types (text, mask) have sidebar editing in ConfigSidebar.tsx
- Users expect consistent editing patterns across track types

## Proposed Changes

### 1. Add Overlay Selection Support in ConfigSidebar

**File:** `apps/desktop/src/routes/editor/ConfigSidebar.tsx`

Add a new section that appears when an overlay segment is selected:
- Show overlay type (Split/FullScreen)
- Show list of items with their delays
- Allow editing item text, style, delay inline
- Add/remove items buttons

### 2. Selection State Integration

**File:** `apps/desktop/src/routes/editor/editorState.ts` (or equivalent)

Verify that overlay selection state is properly exposed:
- `editorState.timeline.selection.type === 'overlay'`
- `editorState.timeline.selection.index` gives the overlay index

### 3. Project Actions for Overlay Mutations

**File:** `apps/desktop/src/routes/editor/projectConfig.ts` (or equivalent)

Ensure these actions exist (or add them):
- `updateOverlayItem(overlayIndex, itemIndex, updates)`
- `addOverlayItem(overlayIndex, item)`
- `removeOverlayItem(overlayIndex, itemIndex)`
- `updateOverlayType(overlayIndex, type)`
- `updateOverlayTiming(overlayIndex, start, end)`

### 4. UI Components Needed

Create or reuse:
- Dropdown for overlay type selection
- Item list with inline editing
- Time input fields for start/end/delay
- Style selector (title/bullet/numbered)

## Implementation Steps

1. **Research Phase**
   - Check how text segment sidebar editing works in ConfigSidebar.tsx
   - Identify existing overlay mutation actions in projectConfig.ts
   - Understand selection state structure

2. **Add Overlay Section to ConfigSidebar**
   - Add conditional render when selection.type === 'overlay'
   - Create OverlaySidebarEditor component
   - Wire up to project actions

3. **Test & Verify**
   - Click overlay in timeline → sidebar shows editor
   - Edit item text → changes saved
   - Add/remove items works
   - Type change works

## Acceptance Criteria

- [ ] Selecting an overlay segment in timeline shows sidebar editor
- [ ] Can edit overlay type (Split/FullScreen) from sidebar
- [ ] Can edit individual item text from sidebar
- [ ] Can edit item style (title/bullet/numbered) from sidebar
- [ ] Can edit item delay from sidebar
- [ ] Can add new items from sidebar
- [ ] Can remove items from sidebar
- [ ] Changes persist to project config
- [ ] Double-click modal still works as alternative

## Files to Modify

1. `apps/desktop/src/routes/editor/ConfigSidebar.tsx` - Add overlay section
2. `apps/desktop/src/routes/editor/projectConfig.ts` - Add/verify mutation actions
3. Possibly create `apps/desktop/src/routes/editor/OverlaySidebarEditor.tsx` - New component

## Risk Assessment

- **Low Risk**: This is additive functionality, doesn't change existing behavior
- **Medium Complexity**: Need to understand existing sidebar patterns and adapt
- **Testing**: Manual testing in editor UI required
