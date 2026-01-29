extends RefCounted
## RedrawThrottle.gd
##
## Shared redraw throttling helper for DataViz/Canvas components.
## 목적: MAX_REDRAW_HZ 계약을 “중복 없이” 구현하여 드리프트를 줄인다.
##
## NOTE:
## - queue_redraw() 호출은 각 컴포넌트의 _request_redraw()에서 수행한다
##   (FIX_2601 Gate C v1: queue_redraw 직접 호출 제한을 유지).

var last_redraw_ms: int = 0
var redraw_scheduled: bool = false
var trigger_count: int = 0


func request_redraw(owner: Node, max_redraw_hz: int, draw_call: Callable) -> void:
	# Unthrottled mode (debug/disabled)
	if max_redraw_hz <= 0:
		trigger_count += 1
		draw_call.call()
		return

	# If not inside tree, timer scheduling is unsafe; request immediately.
	if owner == null or not owner.is_inside_tree():
		trigger_count += 1
		draw_call.call()
		return

	var now: int = Time.get_ticks_msec()
	var min_interval_ms: int = int(ceil(1000.0 / float(max_redraw_hz)))
	min_interval_ms = max(min_interval_ms, 1)

	if now - last_redraw_ms >= min_interval_ms:
		last_redraw_ms = now
		trigger_count += 1
		draw_call.call()
		return

	if redraw_scheduled:
		return

	redraw_scheduled = true
	var delay_sec: float = float(min_interval_ms - (now - last_redraw_ms)) / 1000.0
	owner.get_tree().create_timer(max(delay_sec, 0.0)).timeout.connect(func():
		redraw_scheduled = false
		last_redraw_ms = Time.get_ticks_msec()
		trigger_count += 1
		draw_call.call()
	)

