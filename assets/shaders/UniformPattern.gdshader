shader_type canvas_item;

// 유니폼 텍스처 패턴 쉐이더
// - 소스 색상(빨간색 유니폼)을 패턴 텍스처로 교체
// - 텍스처 타일링 및 팀 컬러 적용

// 패턴 텍스처 (2x2 체커보드 등)
uniform sampler2D pattern_texture : hint_default_white, repeat_enable;

// 패턴 스케일 (숫자가 클수록 패턴이 작게 반복)
uniform float pattern_scale : hint_range(1.0, 64.0) = 8.0;

// 유니폼 영역 소스 색상 (원본 이미지에서 찾을 색상)
uniform vec4 source_uniform_color : source_color = vec4(0.8, 0.15, 0.15, 1.0);

// 색상 매칭 threshold
uniform float threshold : hint_range(0.0, 0.5) = 0.15;

// 패턴 적용 여부 (false면 단색 모드)
uniform bool use_pattern = true;

// 팀 기본 색상 (패턴 미사용 시 또는 패턴에 곱할 색상)
uniform vec4 team_color : source_color = vec4(0.0, 0.0, 1.0, 1.0);

// 패턴 색상 블렌드 강도 (0=팀색만, 1=패턴색만)
uniform float pattern_blend : hint_range(0.0, 1.0) = 0.5;

void fragment() {
    vec4 current_color = texture(TEXTURE, UV);

    // 투명 픽셀 스킵
    if (current_color.a < 0.01) {
        discard;
    }

    // 소스 유니폼 색상과의 거리 계산
    float dist_uniform = distance(current_color.rgb, source_uniform_color.rgb);

    if (dist_uniform < threshold) {
        // 유니폼 영역 감지됨
        if (use_pattern) {
            // 패턴 텍스처 샘플링 (UV 기준 타일링)
            vec2 pattern_uv = UV * pattern_scale;
            vec4 pattern_color = texture(pattern_texture, pattern_uv);

            // 패턴과 팀 색상 블렌딩
            vec3 final_rgb = mix(team_color.rgb, pattern_color.rgb, pattern_blend);

            // 원본 밝기를 유지하여 입체감 보존
            float original_luminance = dot(current_color.rgb, vec3(0.299, 0.587, 0.114));
            float target_luminance = dot(final_rgb, vec3(0.299, 0.587, 0.114));
            float luminance_ratio = original_luminance / max(target_luminance, 0.01);

            // 밝기 보정 적용 (너무 과하지 않게 제한)
            luminance_ratio = clamp(luminance_ratio, 0.7, 1.3);
            final_rgb *= luminance_ratio;

            COLOR = vec4(final_rgb, current_color.a);
        } else {
            // 단색 모드
            COLOR = vec4(team_color.rgb, current_color.a);
        }
    } else {
        // 유니폼 영역 아님 - 원본 유지
        COLOR = current_color;
    }
}
