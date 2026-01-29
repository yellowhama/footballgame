# 게임 엔진 코드 리뷰 전담 위원회 리포트 (2026-01-28)

## 입력
- 코드 범위(실제 근거 기반으로 본 리포트가 다룬 파일/모듈)
  - Godot↔Rust 경계/게이트웨이: `bridge/match_gateway.gd`, `bridge/match_gateway_policy.gd`, `bridge/match_gateway_factory.gd`, `bridge/match_gateway_mock.gd`, `scenes/match/services/match_gateway.gd`, `scenes/match/MatchScreen.gd`, `scenes/match/MatchScreen.tscn`
  - 실시간 세션(틱 파이프라인): `scripts/match_pipeline/MatchSessionController.gd`, `scripts/match_pipeline/PositionSnapshotAdapter.gd`
  - 매치 실행/시드/SSOT 래퍼: `autoload/domain/MatchManager.gd`, `autoload/OpenFootballAPI.gd`, `autoload/rust/FootballRustEngine.gd`, `autoload/rust/TacticalEngine.gd`
  - 저장/상태 집계: `autoload/core/SaveManager.gd`
  - Rust(GDExtension + 코어 엔진): `godot_extension/src/lib.rs`, `crates/of_core/src/engine/live_match.rs`
- 게임 장르/특성: 축구 시뮬(배치 시뮬 + 실시간 뷰어/세션 틱 + 전술 변경 + 리플레이/포지션 트래킹)
- 현재 증상 또는 우려: NONE (입력 없음)

---

## [Layer 1] 아키텍처 / 구조 레이어
관점: “이 구조가 6개월 뒤에도 버틸 수 있는가?”

### ❌ 구조적 위험 Top 3
1) **엔진 인스턴스(=FootballMatchSimulator) 생성 경로가 여러 갈래로 분기되어 ‘단일 권위 인스턴스’가 깨짐**
   - 근거:
     - `autoload/rust/FootballRustEngine.gd`에서 `_rust_simulator = ClassDB.instantiate("FootballMatchSimulator")`로 인스턴스 보유(권위 엔진)  
     - `autoload/domain/StageManager.gd:45-50`에서 `_get_match_simulator()`가 매 호출마다 새 인스턴스 생성  
     - `bridge/match_gateway.gd:23`에서 새 인스턴스 생성  
     - `scenes/match/services/match_gateway.gd:12-16`에서 새 인스턴스 생성  
     - `scenes/match/MatchScreen.gd:25`에서 Factory로 또 다른 게이트웨이를 트리 하위에 추가(결과적으로 게이트웨이/시뮬레이터 중복)
   - 왜 문제인지:
     - “세션(stateful) API(라이브 세션/전술 변경/finish)”와 “배치 시뮬(stateless)”가 같은 클래스에 공존하는 구조에서, 호출자마다 서로 다른 인스턴스를 쓰면 **세션 상태가 어디에 붙어있는지 추적 불가능**해진다.
     - 같은 프로젝트 안에서 “/root의 권위 엔진”과 “씬 로컬 엔진”이 동시에 존재하면, UI/매니저가 어떤 인스턴스에 호출했는지에 따라 결과가 달라지고, 그 차이가 로그로도 드러나지 않는다(인스턴스 id만 다름).
   - 어디서 터질지:
     - 세션 기반 호출이 권위 엔진이 아닌 인스턴스에 들어가면 Rust 쪽은 **“No match session active”**로 종료(예: `godot_extension/src/lib.rs:5337-5349`에서 세션 없음 처리).
     - 반대로 “전술 변경”이 다른 인스턴스에 적용되면, 뷰어는 “전술 변경 UI는 눌렸는데 경기에는 반영이 안 되는” 형태로 유저가 먼저 발견(일관 재현 어려움: 씬 로딩/인스턴스 생성 순서 의존).

2) **경계(Bridge) 계약이 ‘필수 메서드만’ 검증되고, 실제 사용 엔트리포인트는 검증에서 빠져 드리프트가 바로 런타임으로 새어 나감**
   - 근거:
     - `autoload/rust/FootballRustEngine.gd:112-123`는 필수 메서드를 `simulate_match_from_setup`, `simulate_match_from_binary`로만 검증.
     - 그런데 게이트웨이/씬 서비스는 `simulate_match_json_budget`를 호출:
       - `bridge/match_gateway.gd:78`
       - `scenes/match/services/match_gateway.gd:20-23`
     - Rust GDExtension 측에 노출된 예산 엔트리포인트는 `simulate_match_with_budget`임: `godot_extension/src/lib.rs:1306-1345`
   - 왜 문제인지:
     - “검증은 통과(필수 메서드 존재)하지만 실제로 쓰는 메서드가 없음” 상태가 가능해져, 실패가 **부팅 시점이 아니라 기능 실행 시점**으로 밀린다.
     - 실패가 `null/""` 같은 값으로 내려오면(`bridge/match_gateway.gd:80-88`) 상위 레이어는 재시도/부분결과로 삼켜서, 결함이 분산되고 원인 추적이 더 어려워진다.
   - 어디서 터질지:
     - 경기 화면/서비스가 Rust 호출이 된다고 믿고 이벤트를 버스에 흘려보내지만, 실제로는 mock/partial만 돌아오는 상태가 고착(“가끔 경기 결과가 비거나 이벤트가 0개”로 표출).

3) **저장(SaveManager)이 ‘모든 시스템의 상태’를 직접 수집하는 중앙 집계기로 커지며, 부트/로딩/엔진 준비 상태에 강결합됨**
   - 근거:
     - `autoload/core/SaveManager.gd:80-235`에서 여러 매니저/서비스에 직접 접근해 payload를 합성(일종의 “글로벌 스냅샷 빌더”).
     - `autoload/core/SaveManager.gd:239-242`에서 `FootballRustEngine`이 준비되지 않으면 저장을 즉시 중단.
     - `autoload/core/SaveManager.gd:221-229`에서 `MyTeamData._uid_sequence_counter` 같은 내부 필드에 직접 접근(캡슐화 파괴).
   - 왜 문제인지:
     - 저장 포맷/로드 포맷의 변경이 “각 시스템”이 아니라 “SaveManager”에 집중되어, 확장할수록 **저장 관련 결함이 단일 지점에서 폭발**한다.
     - Rust 엔진 준비 실패(플랫폼/배포/확장 로딩 실패)가 곧 저장 불능으로 이어지면, 게임 진행과 무관한 이유로 데이터 유실 리스크가 생긴다.
   - 어디서 터질지:
     - GDExtension 미로드/버전 불일치/초기화 실패 상황에서 저장 버튼이 동작하지 않는 형태로 유저가 즉시 발견(특히 출시/패치 직후, DLL/플러그인 배포 이슈에서 치명적).

### 가장 먼저 터질 포인트 1곳 (구체적 이유 포함)
- **`scenes/match/MatchScreen.gd:65`에서 `simulate_json()`을 `await` 없이 호출**
  - 근거:
    - Factory가 반환하는 구현은 `bridge/match_gateway.gd` 또는 `bridge/match_gateway_mock.gd`인데 둘 다 내부에서 `await create_timer(...)`를 사용한다(`bridge/match_gateway.gd:83-84`, `bridge/match_gateway_mock.gd`의 지연 시뮬).
  - 왜 문제인지:
    - GDScript에서 `await`가 있는 함수는 **상황에 따라 즉시 `Dictionary`가 아니라 `GDScriptFunctionState`를 반환**한다(첫 `await`까지 실행 후 중단).
  - 어디서 터질지:
    - `scenes/match/MatchScreen.gd:67`의 `result.has("events")`에서 `result`가 Dictionary가 아닐 때 타입 오류/런타임 에러.
    - “항상”이 아니라 **지연이 발생하는 조건에서만** 터져 재현이 어렵다(예: 재시도 백오프가 걸린 경우, mock이 10ms 초과 delay를 태운 경우).

---

## [Layer 2] 게임 로직 / 시뮬레이션 레이어
관점: “이건 ‘게임 규칙’인가, 우연의 집합인가?”

### ❌ 규칙이 붕괴되는 지점
1) **벽시계(wall-clock) 예산 기반 시뮬레이션은 입력 동일성을 ‘실행 환경’에 종속시킴**
   - 근거: `godot_extension/src/lib.rs:1306-1337`에서 `SimBudget::new(max_wall_ms, ...)`를 구성하고 `simulate_match_json_budget()` 호출.
   - 왜 문제인지:
     - “같은 입력이라도 CPU/부하에 따라 wall-time 예산이 먼저 소진되면 결과가 달라지는” 구조가 된다. 즉, **동일 입력→동일 결과의 전제 자체가 깨진다.**
   - 어디서 터질지:
     - 느린 기기/백그라운드 부하/디버그 빌드에서 이벤트 수/분 단위 제한에 걸려 partial 결과가 나오고, 그 결과가 저장/통계에 반영되면 이후 진행이 비가역적으로 갈라진다.

2) **시드(seed) 생성/주입 경로가 여러 곳에 흩어져 있고, 일부는 전역 RNG/시간에 의존**
   - 근거:
     - `autoload/domain/MatchManager.gd:157-162`에서 seed 미지정 시 `Time.get_ticks_usec()`로 대체.
     - `autoload/OpenFootballAPI.gd:2366-2368`에서 `randi()` 기반 seed 생성.
     - `autoload/rust/FootballRustEngine.gd:262-265`에서 match_data seed 미지정 시 `Time.get_ticks_usec()` 사용.
   - 왜 문제인지:
     - “seed를 SSOT로 강제해야 하는 레이어”가 분산되어 있어, 어떤 호출 경로를 타느냐에 따라 seed가 바뀐다.
     - 전역 RNG(`randi()`)는 엔진 초기 seed/호출 순서에 영향을 받으므로, 다른 시스템의 랜덤 호출이 끼어들면 같은 입력이더라도 seed가 달라질 수 있다.
   - 어디서 터질지:
     - 리플레이/되감기/버그 리포트에서 “seed가 같다고 생각했는데 결과가 다름” 혹은 “seed를 로그에 찍지 않아 재현 불가”로 이어진다.

3) **세션 틱 속도/시간계는 Godot 쪽 설정과 Rust 쪽 SSOT가 어긋나 ‘규칙(시간)’이 갈라짐**
   - 근거:
     - Godot: `scripts/match_pipeline/MatchSessionController.gd`는 `_max_dt_ms`를 변경해 속도를 조절하고(`set_speed()`), 이를 step 호출 인자로 전달.
     - Rust: `godot_extension/src/lib.rs:5329-5331` 및 `:5537-5538`에서 `max_dt_ms`를 **명시적으로 무시**(고정 250ms tick)한다고 선언.
     - Core: `crates/of_core/src/engine/live_match.rs:16-21`에서 `MS_PER_TICK = 250` 고정.
   - 왜 문제인지:
     - “시간이 어떻게 흐르는가”는 룰의 일부인데, Godot 레이어는 `_max_dt_ms`를 시간 룰로 취급하는 반면 Rust는 호출 빈도를 시간 룰로 취급한다.
   - 어디서 터질지:
     - 배속/저속/프레임 드랍에서 “뷰어 시간(t_ms)이 체감 속도와 다르게 흐름”, “하프타임/교체 타이밍이 UI 기준과 어긋남” 같은 불일치로 표면화.

### ⚠️ 재현 불가능 버그가 생길 조건
- **예산(벽시계) 기반 엔진 호출 + 동일 입력을 ‘저장된 seed’ 없이 재실행**: 기기 성능/부하에 따라 partial 경계가 달라져 결과가 흔들림(`godot_extension/src/lib.rs:1306-1337`).
- **전역 RNG(`randi()`)에 의존한 seed 생성이 다른 랜덤 호출과 섞임**: 호출 순서가 바뀌는 순간 seed가 달라져 동일 저장 상태에서도 결과가 바뀜(`autoload/OpenFootballAPI.gd:2366-2368`).
- **async 함수 미-await 호출**: 지연이 걸리는 날/상황에서만 반환 타입이 달라져, 로그만 남기고 사라지는 크래시/오류로 변질(`scenes/match/MatchScreen.gd:65-69`).

### ✅ 규칙을 SSOT로 묶어야 할 대상
- **Seed 정책/저장 규약**: “누가/언제/어떤 호출 경로에서도 seed를 생성하지 않고 반드시 상위에서 주입 + 저장”으로 통일(현재는 `MatchManager`, `OpenFootballAPI`, `FootballRustEngine`에 분산).
- **TeamInstructions 스키마(문자열 enum/alias 매핑)**: `autoload/rust/TacticalEngine.gd`의 매핑 로직과 `autoload/rust/FootballRustEngine.gd`의 바이너리 인코딩이 서로 다른 입력 허용/기본값을 갖지 않게, 단일 스펙/검증기로 고정.
- **세션 틱 시간 모델**: “호출 빈도 기반” vs “dt 인자 기반” 중 하나로 결정하고 계약화(현재는 Godot/ Rust가 서로 다른 모델).
- **Hero Time(유저 개입) 신호 스키마**: Godot는 `paused/user_decision`을 기대하지만 Rust 변환 딕셔너리에 해당 키가 없음(세션 UI가 ‘멈출 조건’을 못 받는 구조).

---

## [Layer 3] ⏱️ 성능 / 스케일 레이어
관점: “지금은 되는데, 판 수/유닛 수 늘면 죽는가?”

### ❌ 병목 후보 Top 3
1) **포지션 트랙에서 프레임 검색이 매 호출 O(N) (재생 전체는 O(N²))**
   - 근거: `scripts/match_pipeline/PositionSnapshotAdapter.gd:349-425`의 `_find_frame(track, time_ms)`가 매번 `for entry in track:`로 전체 스캔.
   - 왜 문제인지:
     - 재생/스크러빙이 `time_ms`를 연속적으로 증가시키는 패턴이라면, “이전 위치” 정보를 활용할 수 있는데 매번 전체를 훑는다.
   - 어디서 터질지:
     - 포지션 샘플 수가 늘어날수록(긴 경기, 더 촘촘한 샘플링, 저장된 리플레이 재생) 프레임 드랍/입력 지연이 먼저 발생.

2) **세이브 시 대용량 `match_position_data`를 그대로 포함 + 샘플 카운팅을 위해 전체 순회**
   - 근거:
     - 수집: `autoload/core/SaveManager.gd:191-214`에서 `match_position_data`를 unified_data에 포함.
     - 카운팅: `autoload/core/SaveManager.gd:203-207`에서 `players` 딕셔너리의 모든 키/엔트리를 순회해 샘플 수 합산.
   - 왜 문제인지:
     - “저장 빈도(주간 자동 저장) × 데이터 크기(포지션 샘플)”가 곧 프레임 스톨로 이어진다.
   - 어디서 터질지:
     - 시즌이 진행될수록 세이브 파일이 커지고, 저장 시점에 UI가 멈추거나(모바일/저사양) 저장 실패로 이어진다.

3) **실시간 세션 틱에서 프레임당 다중 step + 스냅샷 정규화/로스터 스왑이 합쳐져 비용이 누적**
   - 근거:
     - `scripts/match_pipeline/MatchSessionController.gd:184-208`에서 프레임 드랍 시 최대 `MAX_STEPS_PER_FRAME`(4)번 step 호출.
     - 매 step마다 `_apply_substitution_events_to_rosters()` + `PositionSnapshotAdapter.from_step_*_normalized()` 수행(`scripts/match_pipeline/MatchSessionController.gd:236-251`).
   - 왜 문제인지:
     - “프레임 드랍이 발생하면 더 많은 작업을 같은 프레임에 몰아넣는” 구조라서, 임계점 넘어가면 회복이 아니라 **붕괴(spiral of death)**로 갈 수 있다.
   - 어디서 터질지:
     - 이벤트가 많은 틱/로스터 변화 틱에서 CPU 스파이크 → 프레임 드랍 → 한 프레임에 더 많은 step → 더 큰 스파이크로 악화.

### 스케일 시 가장 먼저 무너질 루프
- `scripts/match_pipeline/PositionSnapshotAdapter.gd:349`의 `_find_frame()`  
  - 왜: “시간 증가형 접근(재생)”에서 선형 전체 스캔은 샘플 수에 정비례로 커지며, 재생 전체 비용이 누적되어 O(N²)로 확대된다.  
  - 어디서: 리플레이 타임라인을 길게 재생하거나, 저장/로드 후 포지션 데이터를 계속 참조하는 UI에서 가장 먼저 프레임이 무너진다.

---

## [Layer 4] 버그 유발 패턴 / 사고 지점
관점: “QA가 아니라 유저가 먼저 발견할 버그는?”

### ☠️ 재현 어려운 치명 버그 시나리오 2개
1) **async 반환 타입 흔들림으로 인한 ‘가끔만’ 터지는 경기 화면 오류**
   - 조건(왜 문제인지):
     - `simulate_json()` 내부가 `await`를 타는 순간 반환이 `Dictionary`가 아니라 `FunctionState`로 바뀐다.
   - 근거(어디):
     - 호출부: `scenes/match/MatchScreen.gd:65-69` (`await` 없이 사용)
     - 구현부: `bridge/match_gateway.gd:83-84`(재시도 백오프), `bridge/match_gateway_mock.gd`(지연 시뮬)
   - 어디서 터질지:
     - 특정 환경(재시도 발생/지연 발생)에서만 `result.has(...)` 같은 딕셔너리 API 호출이 실패 → “유저 환경에서만 크래시/오작동”으로 보고된다.

2) **다중 시뮬레이터 인스턴스 때문에 ‘세션이 없는 엔진’에 step/전술 변경이 들어가는 유령 상태**
   - 조건(왜 문제인지):
     - 세션은 한 인스턴스에서 시작했는데, 다른 인스턴스에 대해 `step_match_session`/`change_live_tactic`를 호출하면 상태가 맞지 않는다.
   - 근거(어디):
     - 인스턴스 분산: `autoload/domain/StageManager.gd:45-50`, `bridge/match_gateway.gd:23`, `scenes/match/services/match_gateway.gd:12-16`, `autoload/rust/FootballRustEngine.gd`의 권위 인스턴스.
     - “세션 없음” 처리: `godot_extension/src/lib.rs:5337-5349`가 `error="No match session active"`로 종료.
   - 어디서 터질지:
     - 특정 로딩 순서/씬 재진입/디버그 도구가 엔진을 먼저 생성했을 때만 발생 → “간헐적으로 세션이 시작되지 않음/중간에 끊김/전술이 적용되지 않음”으로 유저가 발견.

### 다시는 안 생기게 막는 구조적 해결책
- **FootballMatchSimulator 단일 인스턴스 강제 + DI 경로 단일화**
  - “모든 Rust 호출은 `/root/FootballRustEngine`이 제공하는 핸들로만”이라는 룰을 코드로 강제(직접 `ClassDB.instantiate("FootballMatchSimulator")` 호출 제거/금지).
  - 씬 로컬 게이트웨이(`scenes/match/services/match_gateway.gd`)와 브리지 게이트웨이(`bridge/match_gateway.gd`)를 단일 구현으로 합치고, 인터페이스(메서드 목록/반환 타입)를 고정.
- **경계 계약(메서드 이름/시그니처/반환 타입) 부팅 시점 검증 확대**
  - 현재 `FootballRustEngine._verify_boundary_contract()`가 검증하지 않는 실사용 엔트리포인트(예산/세션/전술/저장)를 포함해 “부팅 시 실패”로 당긴다.
- **async API를 ‘반드시 await’하는 규약으로 통일하거나, 아예 async를 제거하고 동기 반환만 허용**
  - 호출자 실수로 타입이 흔들리지 않도록, 게이트웨이 레이어에서 “항상 Dictionary 반환(내부에서 await를 금지)” 또는 “항상 await required(호출부 강제)” 중 하나로 정리.

---

## [Layer 5] 리팩토링 우선순위 레이어
관점: “지금 고치면 미래 비용을 얼마나 줄이는가?”

1. P0 (지금 안 고치면 설계 부채 확정)
   - 대상: **게이트웨이/엔진 인스턴스 단일화 + 메서드 계약 드리프트 제거**
   - 왜 지금인가:
     - 현재는 같은 이름의 “MatchGateway”가 서로 다른 파일에 존재하고(`bridge/*`, `scenes/match/services/*`), 서로 다른 방식으로 Rust를 호출하며, 심지어 존재하지 않는 메서드를 호출한다.
   - 안 고치면 어떤 종류의 버그/한계로 변하는가:
     - 세션/전술/저장 등 stateful 기능이 추가될수록 “어떤 인스턴스가 권위인가” 문제가 치명적으로 커지고, 간헐/환경 의존/재현 불가 버그가 상수처럼 따라붙는다.

2. P1 (다음 확장 전에 필수)
   - 대상: **결정론 SSOT(Seed/시간/전술 스키마) + wall-clock 예산의 게임플레이 분리**
   - 왜 지금인가:
     - 리플레이/통계/랭킹/동일 경기 재현 같은 확장 요구가 들어오는 순간, seed와 시간 모델이 분산된 구조는 바로 한계에 부딪힌다.
   - 안 고치면 어떤 종류의 버그/한계로 변하는가:
     - 같은 저장/같은 입력인데 결과가 달라지는 “규칙 붕괴”가 발생하고, 버그 리포트/테스트 자동화/회귀 테스트가 사실상 불가능해진다.

3. P2 (시간 날 때)
   - 대상: **리플레이/포지션 데이터 접근 경로 최적화 + 저장 페이로드 슬림화**
   - 왜 지금인가:
     - 기능이 늘면서 포지션 트래킹/리플레이가 길어지면 `_find_frame()` 같은 선형 스캔이 가장 먼저 사용자 경험을 무너뜨린다.
   - 안 고치면 어떤 종류의 버그/한계로 변하는가:
     - 특정 경기/특정 시즌부터 UI 프레임 드랍이 급격히 증가하고, 저장 시 멈춤/실패가 누적되어 “진행이 쌓일수록 더 느려지는” 제품 특성을 갖게 된다.

---

## [최종 요약]
- 이 코드는 지금 기준으로
  - [ ] 실험용
  - [x] MVP용
  - [ ] 라이브 서비스 가능

- “이 구조의 가장 큰 거짓말 1줄로 요약”
  - **“Rust가 SSOT라고 말하지만, Godot 레이어가 언제든 새 `FootballMatchSimulator`를 만들어 다른 SSOT를 생성할 수 있다.”**

---

# [부록] P0 실행 설계 (A/B/C) — 2026-01-29 추가
이 섹션은 본 리포트의 P0 권고사항을 “실제로 막히게” 만드는 **SSOT/게이트/정적 검사**를 정의한다.

## (A) “단일 권위 엔진 인스턴스” 강제 (정적 규칙 + CI 게이트)

### A-0. 목표 (Fail-Closed)
- `FootballMatchSimulator` 인스턴스는 **오직** `autoload/rust/FootballRustEngine.gd`에서만 생성한다.
- 그 외 파일에서의 직접 생성은 **빌드/CI에서 즉시 실패**시킨다.

### A-1. 하드 금지 규칙 (R1)
- 금지 패턴(리터럴):
  - `ClassDB.instantiate("FootballMatchSimulator")`
  - `FootballMatchSimulator.new(` (프로젝트에 존재할 수 있는 변형)
- 허용(화이트리스트):
  - `autoload/rust/FootballRustEngine.gd` (단 1곳)

### A-2. 현재 코드 기준 “직접 생성” 히트(추적 포인트)
본 리포트에 포함된 근거 외에도, 아래 파일들은 현재 직접 인스턴스 생성을 포함한다(즉시 정리/차단 대상).
- `autoload/domain/StageManager.gd`
- `bridge/match_gateway.gd`
- `scenes/match/services/match_gateway.gd`
- `scripts/ui/DeckSelectionPopup.gd`
- `scripts/screens/PersonalityDemo.gd`

### A-3. CI Gate 정의
- Gate 이름: `ci_gate_single_engine_instance`
- 동작:
  1. repo 전체에서 금지 패턴을 스캔한다.
  2. 화이트리스트 파일(1개)에서만 허용한다.
  3. 그 외 발견 시 `E_SINGLETON_ENGINE_INSTANTIATION_FORBIDDEN`로 실패한다.
- 스크립트 경로(템플릿):
  - `tools/ci/ci_gate_single_engine_instance.ps1`
  - `tools/ci/ci_gate_single_engine_instance.sh`

### A-4. 런타임 방어(선택, 디버그 빌드 권장)
- `FootballRustEngine`가 `_rust_simulator.get_instance_id()`를 항상 로그로 남기고,
- “세션 기반 API 호출” 시에도 동일 id가 로그에 찍히도록(또는 wrapper에서 포함) 해서 “유령 세션/유령 전술”을 빠르게 추적 가능하게 한다.

---

## (B) Bridge 계약 SSOT + “부팅 시점 Fail-Closed 게이트”

### B-0. 목표
- “부팅은 되는데 기능 실행에서 터지는” 계약 드리프트를 없애기 위해,
  - **SSOT 1개**에 “실사용 엔트리포인트”를 고정하고
  - 부팅 시점에 **전수 검증**(Fail-Closed)한다.

### B-1. SSOT 파일 (단일 근거)
- 경로: `docs/ssot/BRIDGE_CONTRACT_SSOT.json`
- 최소 포함 필드:
  - `bridge_class`, `version`, `methods[]`
  - `methods[].name`, `methods[].args`, `methods[].return`
  - `methods[].success_shape`, `methods[].error_shape`
  - `methods[].used_by` (어느 GD가 호출하는지)

### B-2. “실사용 엔트리포인트” 최소 리스트(리포트 기준)
아래는 현 구조에서 “검증에 포함되지 않으면 런타임 유출”이 가능한 대표 엔트리포인트다.
- 배치/예산:
  - `simulate_match_with_budget` (Rust 노출명, 권장 SSOT)
  - `simulate_match_json_budget` (현재 게이트웨이 호출명: 드리프트 탐지 대상으로 포함)
- 배치/기본:
  - `simulate_match_json`
  - `simulate_match_with_instructions`
  - `simulate_match_v2_json`
  - `simulate_match_from_setup` (Phase17 canonical)
  - `simulate_match_from_binary` (MRB0/OFRP binary replay path)
- 실시간 세션(stateful):
  - `start_match_session`
  - `step_match_session`
  - `finish_match_session`
  - `change_live_tactic`
  - `change_formation_live_match` (전술 엔진 경유로 호출 가능)

### B-3. 부팅 게이트(autoload) 동작 규칙
- 위치: `autoload/rust/FootballRustEngine.gd` 초기화 흐름(엔진 준비 직전)
- 체크:
  1. SSOT에 정의된 메서드 전부 `has_method(name)` 통과
  2. 반환 타입 검증(예: `String`이면 JSON parse 가능, `Dictionary`면 필수 키 존재)
  3. 실패 반환을 “빈 문자열/빈 딕셔너리”로 위장하는 케이스를 금지하고 `error_shape`로 통일
- 실패 시 UX(Fail-Closed):
  - “엔진 준비 실패”로 처리하고 매치/시뮬 기능 진입을 막는다.
  - 디버그 빌드에서는 mismatch 상세를 노출(어떤 메서드/타입이 틀렸는지).

### B-4. CI Gate(정적 검사) — “used_by가 호출하는 메서드가 SSOT에 없다”를 차단
- Gate 이름: `ci_gate_bridge_contract_used_by`
- 동작:
  - `BRIDGE_CONTRACT_SSOT.json`의 `used_by` 목록을 따라가서
  - `.simulate_*`, `.call("...")` 등 메서드 호출명을 스캔
  - SSOT에 없는 호출명이 발견되면 `E_BRIDGE_CONTRACT_USED_BY_DRIFT`로 실패
- 스크립트 경로(템플릿):
  - `tools/ci/ci_gate_bridge_contract_used_by.ps1`

---

## (C) Seed/Time/예산 모델 SSOT “한 장짜리 규약” (Determinism)

### C-0. 목표
- 리플레이/버그 재현/회귀 테스트를 가능하게 만드는 최소 조건을 “문서 + 코드 계약”으로 고정한다.

### C-1. Seed SSOT 규칙
1. seed는 단 1곳(상위 오케스트레이터)에서만 생성한다.
2. 하위 레이어(Godot 서비스/Bridge/Rust core)는 **seed 생성 금지**.
3. 모든 경기 실행 경로는 seed를 **명시적으로 주입**해야 한다(없으면 실패).

금지 패턴(예):
- `Time.get_ticks_usec()` 기반 seed 생성
- `randi()` 기반 seed 생성
- “seed 없으면 내부에서 fallback 생성”

저장/리플레이 최소 메타데이터:
- `seed`, `schema_version`, `engine_build_tag`, `budget_mode`, `result_partial`

### C-2. Time 모델 SSOT 규칙
1. 시뮬 시간 권위는 tick 기반(고정 dt)으로 통일한다.
2. UI 배속/프레임 드랍은 “표시/재생(playhead)”만 바꾸고, 시뮬 tick dt 자체는 바꾸지 않는다.
3. Godot 레이어의 `_max_dt_ms` 같은 값이 Rust에서 무시되는 구조라면, 변수/문서가 “시뮬 dt”를 암시하지 않도록 정정한다.

### C-3. wall-clock 예산(budget) 규칙
1. wall-clock budget 기반 결과는 본질적으로 **partial**이 될 수 있다.
2. partial 결과는 저장/진행/영구 통계에 커밋 금지(또는 `result_partial=true`로 분기 처리).
3. 재현이 필요한 모드(리플레이/버그리포트/검증)는 wall-clock budget 경로 사용 금지(또는 strict 모드에서만 허용).

### C-4. 문서화 SSOT
- 경로: `docs/ssot/SEED_TIME_SSOT.md`
