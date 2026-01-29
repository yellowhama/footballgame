# SEED/TIME SSOT v1 (2026-01-29)
이 문서는 “재현 가능한 경기(Determinism)”를 위한 최소 규약(운영 헌법)이다.

## 1) Seed SSOT 규칙
1. seed는 **단 1곳(상위 오케스트레이터)** 에서만 생성한다.
2. 하위 레이어(`OpenFootballAPI`, `FootballRustEngine`, Bridge, Rust core)는 **seed 생성 금지**.
3. 모든 매치 실행 경로는 seed를 **명시적으로 주입**해야 한다(없으면 실패).

### 금지 패턴
- `Time.get_ticks_usec()` / `Time.get_ticks_msec()`로 seed 대체
- `randi()` 기반 seed 생성
- “seed 없으면 내부에서 자동 생성” 같은 fallback

### 저장/리플레이 규칙
- Save에는 최소 `seed + match_id + schema_version + engine_build_tag`를 저장한다.
- Replay에도 seed를 포함한다(또는 replay header/metadata에 포함).

## 2) Time 모델 SSOT 규칙
1. 시뮬 시간의 권위는 **tick 기반(고정 dt)** 이다.
2. UI 배속/프레임 드랍은 **표시/재생(playhead)** 만 바꾸고, 시뮬 tick dt 자체는 바꾸지 않는다.
3. Godot 레이어에서 `_max_dt_ms` 같은 값이 Rust에서 무시되는 구조라면, 해당 값이 “시뮬 dt”를 의미하지 않도록 변수/문서/주석을 정정한다.

## 3) Budget(예산) 규칙
1. wall-clock budget 기반 결과는 본질적으로 **partial**이 될 수 있다.
2. partial 결과는 저장/진행/영구 통계에 **커밋 금지**(또는 `result_partial=true`로 별도 분기).
3. 재현이 필요한 모드(리플레이/버그리포트/검증)는 wall-clock budget 경로 사용 금지(또는 strict 모드에서만 허용).

## 4) 필수 로그(재현성 최소 세트)
- `seed`
- `schema_version`
- `engine_build_tag`
- `budget_mode` (`none|wallclock|steps`)
- `result_partial` (`true|false`)
