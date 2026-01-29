## UIDHelper - Utility for UID format conversion
## Created: 2025-12-27 (Code Review Fix - Task 1)
##
## PURPOSE:
## Provides consistent UID format conversion across the codebase.
## Eliminates duplicate conversion logic in InteractiveMatchSetup, MatchSetupBuilder, MatchManager.
##
## USAGE:
## ```gdscript
## # Preload UIDHelper in your script
## const UIDHelper = preload("res://scripts/utils/UIDHelper.gd")
##
## # Convert UID to GameCache format (integer)
## var cache_uid = UIDHelper.to_cache_format("csv:123")  # → 123
##
## # Convert UID to engine format (csv: prefix)
## var engine_uid = UIDHelper.to_engine_format("123")    # → "csv:123"
##
## # Safe conversion with validation
## var result = UIDHelper.parse_uid("csv:123")
## if result.valid:
##     print(result.cache_uid)   # 123
##     print(result.engine_uid)  # "csv:123"
## ```
## NOTE: This class extends RefCounted for efficiency (static methods only)
## Use const UIDHelper = preload(...) pattern, NOT autoload
extends RefCounted


## Convert UID to GameCache format (integer or original string)
## GameCache expects integer UIDs, not "csv:123" strings
## @param uid: UID string (e.g., "csv:123", "123", "grad:1")
## @returns: Integer UID for GameCache lookup, or original string if invalid
static func to_cache_format(uid: String) -> Variant:
	var trimmed := uid.strip_edges()

	# Handle "csv:123" format
	if trimmed.begins_with("csv:"):
		var id_str := trimmed.substr(4)
		if id_str.length() > 0 and id_str.is_valid_int():
			return int(id_str)
		else:
			push_warning("[UIDHelper] Invalid csv: format: '%s'" % uid)
			return trimmed  # Return original if invalid

	# Handle "grad:123" format (convert to integer)
	if trimmed.begins_with("grad:"):
		var id_str := trimmed.substr(5)
		if id_str.length() > 0 and id_str.is_valid_int():
			return int(id_str)
		else:
			push_warning("[UIDHelper] Invalid grad: format: '%s'" % uid)
			return trimmed

	# Handle pure numeric string
	if trimmed.is_valid_int():
		return int(trimmed)

	# Return original if no conversion possible
	return trimmed


## Convert UID to Rust engine format ("csv:<n>" prefix)
## Rust v2 API requires "csv:" prefix for player lookups
## @param uid: UID string or integer
## @returns: String with "csv:" prefix (e.g., "csv:123")
static func to_engine_format(uid: Variant) -> String:
	var uid_str := str(uid).strip_edges()

	# Already has "csv:" prefix
	if uid_str.begins_with("csv:"):
		return uid_str

	# Pure numeric → add "csv:" prefix
	if uid_str.is_valid_int():
		return "csv:%s" % uid_str

	# "grad:123" → "csv:123"
	if uid_str.begins_with("grad:"):
		var id_str := uid_str.substr(5)
		if id_str.length() > 0 and id_str.is_valid_int():
			return "csv:%s" % id_str
		else:
			push_warning("[UIDHelper] Cannot convert grad: to engine format: '%s'" % uid)
			return "csv:0"  # Fallback

	# Unknown format → return as-is with warning
	push_warning("[UIDHelper] Unknown UID format for engine: '%s'" % uid)
	return uid_str


## Parse UID and return both cache and engine formats
## @param uid: UID string (any format)
## @returns: Dictionary with {valid: bool, cache_uid: Variant, engine_uid: String, original: String}
static func parse_uid(uid: String) -> Dictionary:
	var trimmed := uid.strip_edges()

	# Validate UID is not empty
	if trimmed.is_empty():
		return {"valid": false, "cache_uid": null, "engine_uid": "", "original": uid, "error": "UID is empty"}

	var cache_uid = to_cache_format(trimmed)
	var engine_uid = to_engine_format(trimmed)

	# Check if conversion was successful (cache_uid should be int for valid UIDs)
	var valid := cache_uid is int or trimmed.is_valid_int()

	return {
		"valid": valid,
		"cache_uid": cache_uid,
		"engine_uid": engine_uid,
		"original": trimmed,
		"error": "" if valid else "Invalid UID format"
	}


## Extract numeric ID from UID (without prefix)
## @param uid: UID string (e.g., "csv:123", "grad:456", "789")
## @returns: Integer ID or -1 if invalid
static func extract_numeric_id(uid: String) -> int:
	var result = parse_uid(uid)
	if result.valid and result.cache_uid is int:
		return result.cache_uid
	return -1


## Validate UID format
## @param uid: UID string to validate
## @returns: true if UID is in valid format (csv:, grad:, or numeric)
static func is_valid_format(uid: String) -> bool:
	var result = parse_uid(uid)
	return result.valid


## Get display name from UID (for debugging/logging)
## @param uid: UID string
## @returns: Human-readable string (e.g., "CSV #123", "Graduate #456")
static func to_display_name(uid: String) -> String:
	var trimmed := uid.strip_edges()

	if trimmed.begins_with("csv:"):
		var id_str := trimmed.substr(4)
		return "CSV #%s" % id_str

	if trimmed.begins_with("grad:"):
		var id_str := trimmed.substr(5)
		return "Graduate #%s" % id_str

	if trimmed.is_valid_int():
		return "Player #%s" % trimmed

	return "Unknown (%s)" % trimmed


## Normalize UID for comparison (removes prefix, converts to lowercase)
## @param uid: UID string
## @returns: Normalized string for equality checks
static func normalize_for_comparison(uid: String) -> String:
	var result = parse_uid(uid)
	if result.valid and result.cache_uid is int:
		return str(result.cache_uid)
	return uid.strip_edges().to_lower()


## Convert array of UIDs to cache format
## @param uids: Array of UID strings
## @returns: Array of converted UIDs (integers or strings)
static func batch_to_cache_format(uids: Array) -> Array:
	var converted := []
	for uid in uids:
		converted.append(to_cache_format(str(uid)))
	return converted


## Convert array of UIDs to engine format
## @param uids: Array of UID strings
## @returns: Array of "csv:" prefixed strings
static func batch_to_engine_format(uids: Array) -> Array:
	var converted := []
	for uid in uids:
		converted.append(to_engine_format(uid))
	return converted
