class_name GameConstantsCore
# GameConstants - 게임 상수 정의 (static 클래스)

# 42개 능력치 리스트 (PlayerData.gd와 일치)
static var SKILLS = [
	# Technical Skills (14개)
	"corners",
	"crossing",
	"dribbling",
	"finishing",
	"first_touch",
	"free_kicks",
	"heading",
	"long_shots",
	"long_throws",
	"marking",
	"passing",
	"penalty_kicks",
	"tackling",
	"technique",
	# Mental Skills (14개)
	"aggression",
	"anticipation",
	"bravery",
	"composure",
	"concentration",
	"decisions",
	"determination",
	"flair",
	"leadership",
	"off_the_ball",
	"positioning",
	"teamwork",
	"vision",
	"work_rate",
	# Physical Skills (8개)
	"acceleration",
	"agility",
	"balance",
	"jumping",
	"natural_fitness",
	"pace",
	"stamina",
	"strength",
	# Goalkeeper Skills (6개)
	"aerial_reach",
	"command_of_area",
	"communication",
	"eccentricity",
	"handling",
	"kicking"
]

# 포지션 정의
static var POSITIONS = [
	"GK", "SW", "CB", "LB", "RB", "LWB", "RWB", "DM", "CM", "LM", "RM", "CAM", "LW", "RW", "CF", "ST"
]

# 기타 게임 상수들
static var MAX_PLAYERS_PER_TEAM = 11
static var MATCH_DURATION_MINUTES = 90
static var SEASON_WEEKS = 52
static var MIN_SKILL_VALUE = 1
static var MAX_SKILL_VALUE = 200

# FIFA Team Roster Regulations (Phase 17 - MatchSetup)
static var STARTERS_PER_TEAM: int = 11  # FIFA regulation: 11 starters
static var SUBSTITUTES_PER_TEAM: int = 7  # FIFA regulation: max 7 subs
static var ROSTER_SIZE_PER_TEAM: int = 18  # 11 starters + 7 subs
static var TOTAL_PLAYER_SLOTS: int = 22  # Home 11 + Away 11 (track_id 0-21)

# Track ID Ranges (Phase 17 - MatchSetup)
static var HOME_TRACK_ID_START: int = 0
static var HOME_TRACK_ID_END: int = 10  # 0-10 (11 players)
static var AWAY_TRACK_ID_START: int = 11
static var AWAY_TRACK_ID_END: int = 21  # 11-21 (11 players)
