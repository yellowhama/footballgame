extends Node
## CSV에서 선수 데이터를 로드하여 CorePlayer 형식으로 변환
## Aisaak Windows 경로가 아닌, 현재 프로젝트 리소스 경로(res://data/players_with_pseudonym.csv)를 사용합니다.

class_name PlayerCSVLoader

## CSV 파일 경로
const CSV_PATH = "res://data/players_with_pseudonym.csv"


## Position 문자열을 단순화된 포지션으로 변환
## FM 스타일: "AM (RC), ST (C)" → 첫 번째 주요 포지션만 추출
static func simplify_position(fm_position: String) -> String:
	# 따옴표 제거
	var clean = fm_position.strip_edges().replace('"', "")

	# 쉼표로 분리되어 있으면 첫 번째만 사용
	if "," in clean:
		clean = clean.split(",")[0].strip_edges()

	# 괄호 제거하고 메인 포지션만 추출
	if "(" in clean:
		clean = clean.split("(")[0].strip_edges()

	# 포지션 약어 표준화
	match clean:
		"GK":
			return "GK"
		"D", "DC", "DR", "DL", "DM", "WB", "WBL", "WBR":
			return "DF"
		"M", "MC", "ML", "MR", "DM", "AM":
			return "MF"
		"ST", "AMC", "AML", "AMR":
			return "FW"
		_:
			# 기본값: 첫 글자로 판단
			if clean.begins_with("D"):
				return "DF"
			elif clean.begins_with("M") or clean.begins_with("A"):
				return "MF"
			elif clean.begins_with("S") or clean.begins_with("F"):
				return "FW"
			else:
				return "MF"  # 기본값


## CSV에서 선수 로드 (팀 이름으로 필터링)
## @param team_name: 팀 이름 (예: "Barcelona")
## @param count: 로드할 선수 수 (기본 18명 - 11명 선발 + 7명 교체)
## @param use_pseudonym: true면 PseudoName 사용, false면 실명 사용
static func load_players_by_team(team_name: String, count: int = 18, use_pseudonym: bool = true) -> Array:
	var file = FileAccess.open(CSV_PATH, FileAccess.READ)
	if not file:
		push_error("❌ Failed to open CSV: %s" % CSV_PATH)
		return []

	var players = []
	var _header_line = file.get_line()  # 헤더 스킵 (unused)

	print("📖 Loading players from CSV for team: %s" % team_name)

	while not file.eof_reached():
		var line = file.get_line().strip_edges()
		if line.is_empty():
			continue

		# CSV 파싱 (쉼표로 분리, 하지만 따옴표 안의 쉼표는 무시)
		var fields = parse_csv_line(line)
		if fields.size() < 9:
			continue

		# CSV 구조: Name, Nationality, Team, Position, CA, PA, Age, PseudoName, PseudoTeam
		var csv_team = fields[2].strip_edges()

		# 팀 매칭 (대소문자 무시)
		if csv_team.to_lower() != team_name.to_lower():
			continue

		var player_name = fields[7].strip_edges() if use_pseudonym else fields[0].strip_edges()
		var position = simplify_position(fields[3])
		var ca = int(fields[4])
		var pa = int(fields[5])

		players.append({"name": player_name, "ca": ca, "pa": pa, "position": position, "condition": 1.0})  # 기본 컨디션

		if players.size() >= count:
			break

	file.close()

	print("   ✅ Loaded %d players from %s" % [players.size(), team_name])
	return players


## CSV에서 CA 기준 상위 선수 로드
## @param count: 로드할 선수 수
## @param min_ca: 최소 CA 값 (기본 150)
static func load_top_players(count: int = 18, min_ca: int = 150, use_pseudonym: bool = true) -> Array:
	var file = FileAccess.open(CSV_PATH, FileAccess.READ)
	if not file:
		push_error("❌ Failed to open CSV: %s" % CSV_PATH)
		return []

	var players = []
	var _header_line = file.get_line()  # 헤더 스킵 (unused)

	print("📖 Loading top %d players (CA >= %d)" % [count, min_ca])

	while not file.eof_reached() and players.size() < count:
		var line = file.get_line().strip_edges()
		if line.is_empty():
			continue

		var fields = parse_csv_line(line)
		if fields.size() < 9:
			continue

		var ca = int(fields[4])
		if ca < min_ca:
			continue

		var player_name = fields[7].strip_edges() if use_pseudonym else fields[0].strip_edges()
		var position = simplify_position(fields[3])
		var pa = int(fields[5])

		players.append({"name": player_name, "ca": ca, "pa": pa, "position": position, "condition": 1.0})

	file.close()

	print("   ✅ Loaded %d top players" % players.size())
	return players


## CSV 라인 파싱 (따옴표 안의 쉼표 처리)
static func parse_csv_line(line: String) -> Array:
	var fields = []
	var current_field = ""
	var in_quotes = false

	for i in range(line.length()):
		var c = line[i]

		if c == '"':
			in_quotes = not in_quotes
		elif c == "," and not in_quotes:
			fields.append(current_field)
			current_field = ""
		else:
			current_field += c

	# 마지막 필드 추가
	fields.append(current_field)

	return fields


## 포지션별 균형잡힌 팀 생성 (CSV에서 로드)
## GK 2명, DF 6명, MF 6명, FW 4명
static func create_balanced_team(use_pseudonym: bool = true, min_ca: int = 140) -> Array:
	var file = FileAccess.open(CSV_PATH, FileAccess.READ)
	if not file:
		push_error("❌ Failed to open CSV: %s" % CSV_PATH)
		return []

	var gk_players = []
	var df_players = []
	var mf_players = []
	var fw_players = []

	var _header_line = file.get_line()  # 헤더 스킵 (unused)

	print("📖 Creating balanced team from CSV (CA >= %d)" % min_ca)

	while not file.eof_reached():
		var line = file.get_line().strip_edges()
		if line.is_empty():
			continue

		var fields = parse_csv_line(line)
		if fields.size() < 9:
			continue

		var ca = int(fields[4])
		if ca < min_ca:
			continue

		var player_name = fields[7].strip_edges() if use_pseudonym else fields[0].strip_edges()
		var position = simplify_position(fields[3])
		var pa = int(fields[5])

		var player_data = {"name": player_name, "ca": ca, "pa": pa, "position": position, "condition": 1.0}

		# 포지션별로 분류
		match position:
			"GK":
				if gk_players.size() < 2:
					gk_players.append(player_data)
			"DF":
				if df_players.size() < 6:
					df_players.append(player_data)
			"MF":
				if mf_players.size() < 6:
					mf_players.append(player_data)
			"FW":
				if fw_players.size() < 4:
					fw_players.append(player_data)

		# 모든 포지션이 채워지면 종료
		if gk_players.size() >= 2 and df_players.size() >= 6 and mf_players.size() >= 6 and fw_players.size() >= 4:
			break

	file.close()

	# 팀 구성
	var team = []
	team.append_array(gk_players)
	team.append_array(df_players)
	team.append_array(mf_players)
	team.append_array(fw_players)

	print(
		(
			"   ✅ Created balanced team: GK=%d, DF=%d, MF=%d, FW=%d (Total: %d)"
			% [gk_players.size(), df_players.size(), mf_players.size(), fw_players.size(), team.size()]
		)
	)

	return team
