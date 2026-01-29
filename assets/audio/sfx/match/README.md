# Match Sound Effects (SFX)

This folder contains sound effects for match gameplay.

## Required Files

### Ball Sounds
| File | Description | Duration |
|------|-------------|----------|
| `ball_kick.ogg` | Kick/pass sound | 0.1-0.3s |
| `ball_bounce.ogg` | Ball bouncing on ground | 0.1-0.2s |
| `ball_hit_post.ogg` | Ball hitting goalpost | 0.3-0.5s |

### Goal Sounds
| File | Description | Duration |
|------|-------------|----------|
| `goal_scored.ogg` | Goal announcement jingle | 1-2s |
| `goal_celebration.ogg` | Celebration/fanfare | 2-4s |

### Whistle Sounds
| File | Description | Duration |
|------|-------------|----------|
| `whistle_short.ogg` | Short whistle (foul, offside) | 0.3-0.5s |
| `whistle_long.ogg` | Long whistle (kickoff, half-time) | 0.8-1.2s |
| `whistle_triple.ogg` | Triple whistle (full-time) | 1.5-2s |

### Impact Sounds
| File | Description | Duration |
|------|-------------|----------|
| `tackle.ogg` | Tackle/slide sound | 0.2-0.4s |
| `collision.ogg` | Player collision | 0.2-0.3s |
| `header.ogg` | Header sound | 0.1-0.2s |

### Crowd Sounds
| File | Description | Duration |
|------|-------------|----------|
| `crowd_cheer.ogg` | Crowd cheering | 1-3s |
| `crowd_groan.ogg` | Crowd disappointment | 1-2s |
| `crowd_ambient.ogg` | Background crowd noise (loop) | 5-10s (loop) |

### Card Sounds
| File | Description | Duration |
|------|-------------|----------|
| `card_yellow.ogg` | Yellow card sound | 0.3-0.5s |
| `card_red.ogg` | Red card sound | 0.5-0.8s |

## Specifications

- **Format**: OGG Vorbis (Godot recommended)
- **Bitrate**: 128-192 kbps
- **Sample Rate**: 44100 Hz
- **Channels**: Mono (SFX) or Stereo (ambient)

## Free Sound Resources

- [Freesound.org](https://freesound.org/) - CC licensed sounds
- [ZapSplat](https://www.zapsplat.com/) - Free with attribution
- [Mixkit](https://mixkit.co/free-sound-effects/) - Free for any use
- [Pixabay Audio](https://pixabay.com/sound-effects/) - Royalty-free

## Usage

The `MatchSFXPlayer` class handles these sounds automatically.
Missing files will be logged but won't cause errors.

```gdscript
# In HorizontalMatchViewer, SFX is automatically played for events
# Manual usage example:
_sfx_player.play(MatchSFXPlayer.MatchSFX.BALL_KICK)
_sfx_player.play_for_event("goal")
```

---

*Created: 2025-12-11 (Phase 9)*
