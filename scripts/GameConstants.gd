extends Node
# GameConstants - 게임 상수 정의

# 게임 버전
const GAME_VERSION = "1.0.0"

# 화면 크기
const SCREEN_WIDTH = 1080
const SCREEN_HEIGHT = 1920

# 색상 상수
const COLOR_PRIMARY = Color(0.2, 0.6, 1.0)
const COLOR_SECONDARY = Color(0.8, 0.8, 0.8)
const COLOR_SUCCESS = Color(0.2, 0.8, 0.2)
const COLOR_WARNING = Color(1.0, 0.6, 0.0)
const COLOR_DANGER = Color(0.8, 0.2, 0.2)

# 훈련 상수
const MAX_FATIGUE = 100.0
const MIN_FATIGUE = 0.0
const TRAINING_INTENSITY_LOW = 0.5
const TRAINING_INTENSITY_MEDIUM = 1.0
const TRAINING_INTENSITY_HIGH = 1.5

# 플레이어 상수
const MAX_AGE = 40
const MIN_AGE = 16
const MAX_SKILL_VALUE = 100
const MIN_SKILL_VALUE = 0

# 매치 상수
const MATCH_DURATION = 90  # 분
const HALF_TIME = 45  # 분


func _ready():
	print("[GameConstants] Initialized")
