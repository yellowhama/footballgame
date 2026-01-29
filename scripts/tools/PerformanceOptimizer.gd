extends Node

# Performance Optimizer
# Analyzes and optimizes game performance

class_name PerformanceOptimizer

var performance_metrics: Dictionary = {}
var optimization_enabled: bool = true


func _ready():
	# Start performance monitoring
	_start_performance_monitoring()


func _start_performance_monitoring():
	"""Start monitoring game performance"""
	if optimization_enabled:
		# Monitor every 5 seconds
		var timer = Timer.new()
		timer.wait_time = 5.0
		timer.timeout.connect(_collect_performance_metrics)
		timer.autostart = true
		add_child(timer)


func _collect_performance_metrics():
	"""Collect current performance metrics"""
	var metrics = {
		"fps": Engine.get_frames_per_second(),
		"memory_usage": _get_memory_usage(),
		"node_count": _get_node_count(),
		"draw_calls": _get_draw_calls(),
		"timestamp": Time.get_unix_time_from_system()
	}

	performance_metrics[Time.get_unix_time_from_system()] = metrics

	# Keep only last 100 measurements
	if performance_metrics.size() > 100:
		var keys = performance_metrics.keys()
		keys.sort()
		for i in range(keys.size() - 100):
			performance_metrics.erase(keys[i])

	# Auto-optimize if performance is poor
	_auto_optimize_if_needed(metrics)


func _get_memory_usage() -> int:
	"""Get current memory usage in MB"""
	return int(float(OS.get_static_memory_usage()) / 1024.0 / 1024.0)


func _get_node_count() -> int:
	"""Get total node count in scene tree"""
	return _count_nodes_recursive(get_tree().current_scene)


func _count_nodes_recursive(node: Node) -> int:
	"""Recursively count nodes"""
	var count = 1
	for child in node.get_children():
		count += _count_nodes_recursive(child)
	return count


func _get_draw_calls() -> int:
	"""Get approximate draw calls (Godot doesn't expose this directly)"""
	# This is an approximation based on visible nodes
	var visible_nodes = _count_visible_nodes(get_tree().current_scene)
	return visible_nodes * 2  # Rough estimate


func _count_visible_nodes(node: Node) -> int:
	"""Count visible nodes recursively"""
	var count = 0
	if node is CanvasItem and node.visible:
		count = 1

	for child in node.get_children():
		count += _count_visible_nodes(child)
	return count


func _auto_optimize_if_needed(metrics: Dictionary):
	"""Automatically optimize if performance is poor"""
	var fps = metrics.fps
	var memory = metrics.memory_usage

	# FPS optimization
	if fps < 30:
		_optimize_for_fps()

	# Memory optimization
	if memory > 500:  # More than 500MB
		_optimize_for_memory()


func _optimize_for_fps():
	"""Optimize for better FPS"""
	print("🔧 FPS 최적화 실행...")

	# Reduce update frequency for non-critical systems
	_reduce_update_frequency()

	# Optimize rendering
	_optimize_rendering()


func _optimize_for_memory():
	"""Optimize for memory usage"""
	print("🔧 메모리 최적화 실행...")

	# Clear unused resources
	_clear_unused_resources()

	# Optimize data structures
	_optimize_data_structures()


func _reduce_update_frequency():
	"""Reduce update frequency for non-critical systems"""
	# This would be implemented based on specific systems
	pass


func _optimize_rendering():
	"""Optimize rendering performance"""
	# This would be implemented based on specific rendering needs
	pass


func _clear_unused_resources():
	"""Clear unused resources from memory"""
	# Force garbage collection
	call_deferred("_force_garbage_collection")


func _force_garbage_collection():
	"""Force garbage collection"""
	# This is handled automatically by Godot
	pass


func _optimize_data_structures():
	"""Optimize data structures for better memory usage"""
	# This would be implemented based on specific data structures
	pass


# Performance analysis functions
func get_performance_summary() -> Dictionary:
	"""Get performance summary"""
	if performance_metrics.is_empty():
		return {"error": "No performance data available"}

	var fps_values = []
	var memory_values = []
	var node_counts = []

	for metrics in performance_metrics.values():
		fps_values.append(metrics.fps)
		memory_values.append(metrics.memory_usage)
		node_counts.append(metrics.node_count)

	fps_values.sort()
	memory_values.sort()
	node_counts.sort()

	return {
		"fps":
		{
			"current": fps_values[-1] if fps_values.size() > 0 else 0,
			"average": _calculate_average(fps_values),
			"min": fps_values[0] if fps_values.size() > 0 else 0,
			"max": fps_values[-1] if fps_values.size() > 0 else 0
		},
		"memory":
		{
			"current": memory_values[-1] if memory_values.size() > 0 else 0,
			"average": _calculate_average(memory_values),
			"min": memory_values[0] if memory_values.size() > 0 else 0,
			"max": memory_values[-1] if memory_values.size() > 0 else 0
		},
		"nodes":
		{
			"current": node_counts[-1] if node_counts.size() > 0 else 0,
			"average": _calculate_average(node_counts),
			"min": node_counts[0] if node_counts.size() > 0 else 0,
			"max": node_counts[-1] if node_counts.size() > 0 else 0
		}
	}


func _calculate_average(values: Array) -> float:
	"""Calculate average of values"""
	if values.is_empty():
		return 0.0

	var sum = 0.0
	for value in values:
		sum += value
	return sum / values.size()


func get_performance_recommendations() -> Array:
	"""Get performance optimization recommendations"""
	var recommendations: Array = []
	var summary = get_performance_summary()

	if summary.has("error"):
		return recommendations

	# FPS recommendations
	if summary.fps.average < 30:
		recommendations.append("FPS가 낮습니다. 렌더링 최적화가 필요합니다.")

	if summary.fps.average < 60:
		recommendations.append("FPS를 개선하기 위해 불필요한 노드를 제거하세요.")

	# Memory recommendations
	if summary.memory.average > 500:
		recommendations.append("메모리 사용량이 높습니다. 리소스 정리가 필요합니다.")

	if summary.memory.average > 1000:
		recommendations.append("메모리 사용량이 매우 높습니다. 데이터 구조를 최적화하세요.")

	# Node count recommendations
	if summary.nodes.average > 1000:
		recommendations.append("노드 수가 많습니다. 씬 구조를 단순화하세요.")

	return recommendations


func enable_optimization():
	"""Enable performance optimization"""
	optimization_enabled = true
	print("✅ 성능 최적화 활성화")


func disable_optimization():
	"""Disable performance optimization"""
	optimization_enabled = false
	print("❌ 성능 최적화 비활성화")


func clear_performance_data():
	"""Clear all performance data"""
	performance_metrics.clear()
	print("🗑️ 성능 데이터 초기화")
