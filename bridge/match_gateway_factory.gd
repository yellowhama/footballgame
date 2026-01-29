# Factory for runtime Rust/Mock selection
extends RefCounted

const MatchGatewayPolicy = preload("res://bridge/match_gateway_policy.gd")


# Load gateway implementations
static func _get_rust_gateway():
	return load("res://bridge/match_gateway.gd")


static func _get_mock_gateway():
	return load("res://bridge/match_gateway_mock.gd")


# Main factory method
static func create(parent: Node) -> Node:
	var gateway: Node

	if MatchGatewayPolicy.should_use_rust():
		print("[Factory] Attempting Rust gateway...")
		var RustGateway = _get_rust_gateway()
		gateway = RustGateway.new()
		parent.add_child(gateway)

		# Test Rust health
		await parent.get_tree().process_frame  # Let _ready() complete

		if gateway.has_method("is_healthy") and gateway.is_healthy():
			print("[Factory] ✅ Rust gateway healthy")
			return gateway
		else:
			print("[Factory] ❌ Rust gateway unhealthy, switching to mock")
			parent.remove_child(gateway)
			gateway.queue_free()

	# Fallback to mock
	print("[Factory] Using Mock gateway")
	var MockGateway = _get_mock_gateway()
	gateway = MockGateway.new()
	parent.add_child(gateway)
	return gateway


# Helper for testing - force create specific type
static func create_rust(parent: Node) -> Node:
	var RustGateway = _get_rust_gateway()
	var gateway = RustGateway.new()
	parent.add_child(gateway)
	return gateway


static func create_mock(parent: Node) -> Node:
	var MockGateway = _get_mock_gateway()
	var gateway = MockGateway.new()
	parent.add_child(gateway)
	return gateway
