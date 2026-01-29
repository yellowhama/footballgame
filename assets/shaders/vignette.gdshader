shader_type canvas_item;
/*
 * vignette.gdshader - 가장자리 어둡게 하는 효과
 *
 * 용도: PreviewContainer 배경 가장자리에 자연스러운 그라데이션
 * 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md
 */

uniform float intensity : hint_range(0.0, 1.0) = 0.4;
uniform float softness : hint_range(0.0, 1.0) = 0.5;
uniform vec4 vignette_color : source_color = vec4(0.0, 0.0, 0.0, 1.0);

void fragment() {
    vec2 uv = UV - 0.5;
    float dist = length(uv) * 2.0;
    float vignette = smoothstep(1.0 - softness, 1.0, dist);
    COLOR = vec4(vignette_color.rgb, vignette * intensity);
}
