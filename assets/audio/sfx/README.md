# 사운드 효과 (SFX) 플레이스홀더

이 폴더에 게임 사운드 효과 파일을 추가하세요.

## 필요한 파일 목록

### UI 사운드
- `button_click.ogg` - 버튼 클릭 (짧은 클릭음)
- `button_hover.ogg` - 버튼 호버 (부드러운 소리)
- `dialogue_open.ogg` - 대화창 열기 (우아한 열림 효과음)
- `dialogue_close.ogg` - 대화창 닫기
- `choice_select.ogg` - 선택지 선택 (확정음)

### 이벤트 사운드
- `event_start.ogg` - 이벤트 시작 (주목 끌기)
- `event_end.ogg` - 이벤트 종료
- `affection_up.ogg` - 호감도 증가 (긍정적 사운드)
- `affection_down.ogg` - 호감도 감소 (부정적 사운드)

### 텍스트 사운드
- `text_type.ogg` - 텍스트 타이핑 (짧은 반복음, ~0.03초)
- `text_complete.ogg` - 텍스트 완료

### 알림 사운드
- `notification.ogg` - 일반 알림
- `warning.ogg` - 경고 (낮은 음)
- `success.ogg` - 성공 (높은 음, 밝은 느낌)

## 추천 사양

- **형식**: OGG Vorbis (Godot 권장)
- **비트레이트**: 128 kbps
- **샘플레이트**: 44100 Hz
- **길이**:
  - 버튼/타이핑: 0.05 ~ 0.2초
  - 이벤트: 0.5 ~ 2초
  - 알림: 0.3 ~ 1초

## 현재 상태

모든 사운드 파일이 없는 경우, `SoundEffectPlayer`는 경고 메시지를 출력하고 사운드를 건너뜁니다.

실제 오디오 파일을 추가하면 자동으로 로드됩니다.

## 무료 사운드 리소스

- **Freesound.org**: https://freesound.org/
- **ZapSplat**: https://www.zapsplat.com/
- **Mixkit**: https://mixkit.co/free-sound-effects/
- **Pixabay Audio**: https://pixabay.com/sound-effects/

## 사용 방법

```gdscript
# Autoload로 SoundEffectPlayer 등록 후:
SoundEffectPlayer.play(SoundEffectPlayer.SFX.BUTTON_CLICK)
SoundEffectPlayer.play(SoundEffectPlayer.SFX.AFFECTION_INCREASE, 0.5)  # 볼륨 50%
```
