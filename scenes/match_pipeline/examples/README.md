# MatchTimelineViewer Example Scenes

This folder contains example scenes wired for the unified match visualization pipeline.

**Terminology**
- **Session**: step-based interactive match running (user intervention, “Hero Time”, etc.).
- **Timeline**: playback/seek/analysis over the same event + position stream.

## Scenes

### 1) `HorizontalMatchSessionViewer.tscn` (recommended)
- Purpose: watch a running session or review a finished match with a clean HUD.
- Viewer script: `HorizontalMatchSessionViewerController.gd`
- Uses: `/root/MatchTimelineController` + `/root/UnifiedFramePipeline`

### 2) `TacticalAnalysisViewer.tscn`
- Purpose: post-match tactical analysis with full overlays.
- Viewer script: `TacticalAnalysisViewerController.gd`
- Uses: `MatchTimelineViewer.apply_preset_tactical_analysis()`

### 3) `MinimapViewer.tscn` (legacy/debug)
- Purpose: lightweight minimap-only view for quick checks.

## Record handoff (common pattern)

```gdscript
# Save record + return path
MatchTimelineHolder.set_timeline_data(record, get_tree().current_scene.scene_file_path)

# Open a viewer scene
get_tree().change_scene_to_file("res://scenes/match_pipeline/examples/HorizontalMatchSessionViewer.tscn")
```
