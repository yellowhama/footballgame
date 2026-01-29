#!/usr/bin/env python3
"""
NPC Data Enhancement Script

Adds manager_id, formation, and tactics to stage_teams_safe.json

Usage: python3 enhance_npc_data.py
"""

import json
import os
from typing import Dict, List, Any

# Paths
DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "data")
INPUT_PATH = os.path.join(DATA_DIR, "stage_teams_safe.json")
OUTPUT_PATH = os.path.join(DATA_DIR, "stage_teams_enhanced.json")
MANAGERS_PATH = os.path.join(DATA_DIR, "dummy_managers.json")

# Tactical presets based on team style
TACTICAL_PRESETS = {
    "Attacking": {
        "attacking_intensity": 0.8,
        "defensive_line_height": 0.7,
        "width": 0.75,
        "pressing_trigger": 0.7,
        "tempo": 0.8,
        "directness": 0.6
    },
    "Defensive": {
        "attacking_intensity": 0.3,
        "defensive_line_height": 0.3,
        "width": 0.5,
        "pressing_trigger": 0.3,
        "tempo": 0.4,
        "directness": 0.4
    },
    "Balanced": {
        "attacking_intensity": 0.5,
        "defensive_line_height": 0.5,
        "width": 0.6,
        "pressing_trigger": 0.5,
        "tempo": 0.5,
        "directness": 0.5
    },
    "Possession": {
        "attacking_intensity": 0.5,
        "defensive_line_height": 0.6,
        "width": 0.7,
        "pressing_trigger": 0.6,
        "tempo": 0.3,
        "directness": 0.3
    },
    "Counter": {
        "attacking_intensity": 0.7,
        "defensive_line_height": 0.35,
        "width": 0.5,
        "pressing_trigger": 0.4,
        "tempo": 0.9,
        "directness": 0.85
    },
    "Pressing": {
        "attacking_intensity": 0.7,
        "defensive_line_height": 0.75,
        "width": 0.65,
        "pressing_trigger": 0.9,
        "tempo": 0.7,
        "directness": 0.6
    },
}

# Formation options based on tactical style
STYLE_FORMATIONS = {
    "Attacking": ["T433", "T4231", "T352"],
    "Defensive": ["T541", "T532", "T4141"],
    "Balanced": ["T442", "T4231", "T433"],
    "Possession": ["T433", "T4231", "T4312"],
    "Counter": ["T442", "T4141", "T352"],
    "Pressing": ["T4231", "T433", "T442"],
}


def load_json(path: str) -> Any:
    """Load JSON file"""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(path: str, data: Any) -> None:
    """Save JSON file"""
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent='\t', ensure_ascii=False)


def select_manager_for_ca(avg_ca: float, managers: List[Dict], index: int) -> int:
    """Select appropriate manager based on team CA"""
    if not managers:
        return 1

    # Higher CA teams get better managers
    if avg_ca >= 140:
        tier = 4  # Elite
    elif avg_ca >= 120:
        tier = 3  # Top
    elif avg_ca >= 100:
        tier = 2  # Good
    elif avg_ca >= 80:
        tier = 1  # Average
    else:
        tier = 0  # Basic

    manager_count = len(managers)
    tier_size = max(1, manager_count // 5)
    tier_start = tier * tier_size
    tier_end = min(tier_start + tier_size, manager_count)

    manager_index = tier_start + (index % (tier_end - tier_start))
    return managers[manager_index].get("id", 1)


def determine_tactical_style(avg_ca: float, index: int) -> str:
    """Determine tactical style based on CA and index for variety"""
    styles = list(TACTICAL_PRESETS.keys())

    # Weight distribution based on CA
    if avg_ca >= 140:
        weights = [0.3, 0.05, 0.2, 0.25, 0.1, 0.1]
    elif avg_ca >= 120:
        weights = [0.25, 0.1, 0.25, 0.2, 0.1, 0.1]
    elif avg_ca >= 100:
        weights = [0.15, 0.15, 0.35, 0.15, 0.1, 0.1]
    elif avg_ca >= 80:
        weights = [0.1, 0.2, 0.35, 0.1, 0.15, 0.1]
    else:
        weights = [0.05, 0.35, 0.35, 0.05, 0.15, 0.05]

    # Use index as seed for deterministic selection
    rand_value = ((index * 7919) % 10000) / 10000.0

    cumulative = 0.0
    for i, w in enumerate(weights):
        cumulative += w
        if rand_value <= cumulative:
            return styles[i]

    return "Balanced"


def add_variation(base_tactics: Dict, index: int) -> Dict:
    """Add small random variation to tactics for uniqueness"""
    varied = {}

    for key, base_value in base_tactics.items():
        variation = ((index * 31) % 20 - 10) / 100.0
        varied[key] = max(0.0, min(1.0, base_value + variation))

    return varied


def enhance_team(team: Dict, managers: List[Dict], index: int) -> Dict:
    """Enhance a single team with manager, formation, and tactics"""
    enhanced = team.copy()
    avg_ca = float(team.get("avg_ca", 50.0))

    # Skip if already has enhancement data
    if all(key in team for key in ["manager_id", "tactics", "formation"]):
        return enhanced

    # Assign manager based on team strength tier
    if "manager_id" not in team:
        enhanced["manager_id"] = select_manager_for_ca(avg_ca, managers, index)

    # Determine tactical style
    style = determine_tactical_style(avg_ca, index)

    # Assign formation
    if "formation" not in team:
        formations = STYLE_FORMATIONS.get(style, ["T442"])
        enhanced["formation"] = formations[index % len(formations)]

    # Assign tactics
    if "tactics" not in team:
        base_tactics = TACTICAL_PRESETS.get(style, TACTICAL_PRESETS["Balanced"])
        enhanced["tactics"] = add_variation(base_tactics, index)

    # Add tactical style tag
    enhanced["tactical_style"] = style

    return enhanced


def main():
    print("[NPC Enhancer] Starting NPC data enhancement...")

    # Load managers
    try:
        managers_data = load_json(MANAGERS_PATH)
        managers = managers_data.get("managers", []) if isinstance(managers_data, dict) else []
        print(f"[NPC Enhancer] Loaded {len(managers)} managers")
    except FileNotFoundError:
        print(f"[NPC Enhancer] Warning: {MANAGERS_PATH} not found, using defaults")
        managers = []

    # Load stage teams
    try:
        teams = load_json(INPUT_PATH)
        print(f"[NPC Enhancer] Loaded {len(teams)} teams")
    except FileNotFoundError:
        print(f"[NPC Enhancer] Error: {INPUT_PATH} not found")
        return

    # Enhance each team
    enhanced = []
    style_counts = {}

    for i, team in enumerate(teams):
        enhanced_team = enhance_team(team, managers, i)
        enhanced.append(enhanced_team)

        # Count styles for statistics
        style = enhanced_team.get("tactical_style", "Unknown")
        style_counts[style] = style_counts.get(style, 0) + 1

    # Save enhanced data
    save_json(OUTPUT_PATH, enhanced)

    print(f"[NPC Enhancer] Enhancement complete!")
    print(f"[NPC Enhancer] Output: {OUTPUT_PATH}")
    print(f"[NPC Enhancer] Teams enhanced: {len(enhanced)}")
    print(f"[NPC Enhancer] Style distribution:")
    for style, count in sorted(style_counts.items()):
        print(f"  - {style}: {count} teams ({100*count/len(enhanced):.1f}%)")


if __name__ == "__main__":
    main()
