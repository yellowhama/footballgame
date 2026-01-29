extends Node
## Export service for history data
## Phase 13: Extended Features - Data Export System
## Autoload singleton: ExportService

signal export_started(format: String)
signal export_completed(success: bool, file_path: String)
signal export_failed(error: String)


## Export match records to CSV format
func export_match_records_csv(records: Array, file_path: String) -> bool:
	export_started.emit("CSV")

	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if not file:
		var error_msg = "Failed to create file: %s" % file_path
		export_failed.emit(error_msg)
		return false

	# CSV Header
	file.store_line(
		"Date,Week,Year,Opponent,Result,Score For,Score Against,Goal Difference,Match Type,Tactic,Opponent Rating"
	)

	# CSV Data
	for record in records:
		var date = record.get("date", "")
		var week = record.get("week", 1)
		var year = record.get("year", 1)
		var opponent = _escape_csv(record.get("opponent_name", "Unknown"))
		var result = record.get("result", "draw")
		var score_for = record.get("goals_scored", 0)
		var score_against = record.get("goals_conceded", 0)
		var goal_diff = score_for - score_against
		var match_type = record.get("match_type", "friendly")
		var tactic = _escape_csv(record.get("tactic_used", "Unknown"))
		var opponent_rating = record.get("opponent_rating", 50)

		var line = (
			"%s,%d,%d,%s,%s,%d,%d,%+d,%s,%s,%d"
			% [
				date,
				week,
				year,
				opponent,
				result,
				score_for,
				score_against,
				goal_diff,
				match_type,
				tactic,
				opponent_rating
			]
		)
		file.store_line(line)

	file.close()

	print("[ExportService] Exported %d match records to CSV: %s" % [records.size(), file_path])
	export_completed.emit(true, file_path)
	return true


## Export training records to CSV format
func export_training_records_csv(records: Array, file_path: String) -> bool:
	export_started.emit("CSV")

	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if not file:
		var error_msg = "Failed to create file: %s" % file_path
		export_failed.emit(error_msg)
		return false

	# CSV Header
	file.store_line(
		"Date,Week,Year,Training Name,Training Type,Condition Before,Condition After,Effectiveness,Attribute Changes"
	)

	# CSV Data
	for record in records:
		var date = record.get("date", "")
		var week = record.get("week", 1)
		var year = record.get("year", 1)
		var training_name = _escape_csv(record.get("training_name", "Unknown"))
		var training_type = record.get("training_type", "unknown")
		var condition_before = record.get("condition_before", 100)
		var condition_after = record.get("condition_after", 90)
		var effectiveness = record.get("effectiveness_modifier", 1.0)

		# Format attribute changes
		var changes = record.get("attribute_changes", {})
		var changes_str = ""
		for attr in changes:
			if changes_str != "":
				changes_str += ";"
			changes_str += "%s:%+d" % [attr, changes[attr]]

		changes_str = _escape_csv(changes_str)

		var line = (
			"%s,%d,%d,%s,%s,%.1f,%.1f,%.2f,%s"
			% [
				date,
				week,
				year,
				training_name,
				training_type,
				condition_before,
				condition_after,
				effectiveness,
				changes_str
			]
		)
		file.store_line(line)

	file.close()

	print("[ExportService] Exported %d training records to CSV: %s" % [records.size(), file_path])
	export_completed.emit(true, file_path)
	return true


## Export match records to JSON format
func export_match_records_json(records: Array, file_path: String) -> bool:
	export_started.emit("JSON")

	var data = {
		"export_type": "match_records",
		"export_date": Time.get_datetime_string_from_system(),
		"record_count": records.size(),
		"records": records
	}

	var json_string = JSON.stringify(data, "  ")  # Pretty print with 2-space indent

	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if not file:
		var error_msg = "Failed to create file: %s" % file_path
		export_failed.emit(error_msg)
		return false

	file.store_string(json_string)
	file.close()

	print("[ExportService] Exported %d match records to JSON: %s" % [records.size(), file_path])
	export_completed.emit(true, file_path)
	return true


## Export training records to JSON format
func export_training_records_json(records: Array, file_path: String) -> bool:
	export_started.emit("JSON")

	var data = {
		"export_type": "training_records",
		"export_date": Time.get_datetime_string_from_system(),
		"record_count": records.size(),
		"records": records
	}

	var json_string = JSON.stringify(data, "  ")  # Pretty print with 2-space indent

	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if not file:
		var error_msg = "Failed to create file: %s" % file_path
		export_failed.emit(error_msg)
		return false

	file.store_string(json_string)
	file.close()

	print("[ExportService] Exported %d training records to JSON: %s" % [records.size(), file_path])
	export_completed.emit(true, file_path)
	return true


## Export combined data (matches + training) to JSON
func export_all_records_json(match_records: Array, training_records: Array, file_path: String) -> bool:
	export_started.emit("JSON")

	var data = {
		"export_type": "all_records",
		"export_date": Time.get_datetime_string_from_system(),
		"match_record_count": match_records.size(),
		"training_record_count": training_records.size(),
		"match_records": match_records,
		"training_records": training_records
	}

	var json_string = JSON.stringify(data, "  ")

	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if not file:
		var error_msg = "Failed to create file: %s" % file_path
		export_failed.emit(error_msg)
		return false

	file.store_string(json_string)
	file.close()

	print(
		(
			"[ExportService] Exported %d match + %d training records to JSON: %s"
			% [match_records.size(), training_records.size(), file_path]
		)
	)
	export_completed.emit(true, file_path)
	return true


## Escape CSV special characters
func _escape_csv(value: String) -> String:
	if value.contains(",") or value.contains('"') or value.contains("\n"):
		# Escape double quotes by doubling them
		value = value.replace('"', '""')
		# Wrap in double quotes
		return '"%s"' % value
	return value


## Get suggested filename based on export type and format
func get_suggested_filename(export_type: String, format: String) -> String:
	var date_str = Time.get_datetime_string_from_system().replace(":", "-")
	var type_str = export_type.to_lower().replace(" ", "_")
	var ext = format.to_lower()

	return "football_%s_%s.%s" % [type_str, date_str, ext]


## Validate export path
func validate_export_path(file_path: String) -> Dictionary:
	var result = {"valid": false, "error": ""}

	if file_path == "":
		result.error = "File path is empty"
		return result

	# Check if parent directory exists
	var dir_path = file_path.get_base_dir()
	if not DirAccess.dir_exists_absolute(dir_path):
		result.error = "Directory does not exist: %s" % dir_path
		return result

	# Check if file already exists (warn but allow)
	if FileAccess.file_exists(file_path):
		result.valid = true
		result.error = "File already exists and will be overwritten"
		return result

	result.valid = true
	return result
