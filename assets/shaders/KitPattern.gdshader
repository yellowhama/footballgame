shader_type canvas_item;
/*
 * KitPattern.gdshader - 팀 유니폼 패턴 셰이더
 *
 * 기능:
 *   - 단색, 가로줄, 세로줄, 체크 무늬 실시간 생성
 *   - 기존 ColorSwap 기능 포함 (키 컬러 → 타겟 컬러)
 *
 * 사용법:
 *   1. 스프라이트의 옷 부분이 특정 색(key_color)으로 칠해져 있어야 함
 *   2. primary_color, secondary_color로 팀 컬러 설정
 *   3. pattern_type으로 무늬 종류 선택
 *
 * 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
 */

// --- 키 컬러 설정 ---
uniform vec4 key_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);  // 원본 옷 색상 (기본: 흰색)
uniform float tolerance : hint_range(0.0, 1.0) = 0.1;  // 색상 매칭 허용 범위

// --- 팀 컬러 설정 ---
uniform vec4 primary_color : source_color = vec4(1.0, 0.0, 0.0, 1.0);    // 팀 메인 색
uniform vec4 secondary_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);  // 팀 서브 색

// --- 패턴 설정 ---
// 0: 단색(Solid), 1: 가로줄(Hoops), 2: 세로줄(Stripes), 3: 체크(Checker), 4: 대각선(Diagonal)
uniform int pattern_type : hint_range(0, 4) = 0;
uniform float pattern_scale : hint_range(1.0, 16.0) = 4.0;  // 무늬 크기 (픽셀 단위)
uniform float pattern_ratio : hint_range(0.1, 0.9) = 0.5;   // 메인/서브 비율

// --- 추가 옵션 ---
uniform bool preserve_shading = true;  // 원본의 명암 보존

void fragment() {
    vec4 tex = texture(TEXTURE, UV);

    // 투명 픽셀 스킵
    if (tex.a < 0.01) {
        discard;
    }

    // 현재 픽셀이 '옷(키 컬러)'인지 확인
    float dist = distance(tex.rgb, key_color.rgb);

    if (dist < tolerance) {
        // === 옷 영역 - 패턴 적용 ===

        // 픽셀 좌표 계산 (정수 픽셀 단위로 끊어지게)
        vec2 pixel_coord = UV / TEXTURE_PIXEL_SIZE;
        float pattern_val = 0.0;

        if (pattern_type == 0) {
            // 단색 (Solid) - 메인 컬러만
            pattern_val = 0.0;
        }
        else if (pattern_type == 1) {
            // 가로 줄무늬 (Hoops) - Celtic, QPR 스타일
            pattern_val = step(pattern_ratio, fract(pixel_coord.y / pattern_scale));
        }
        else if (pattern_type == 2) {
            // 세로 줄무늬 (Stripes) - AC Milan, Juventus 스타일
            pattern_val = step(pattern_ratio, fract(pixel_coord.x / pattern_scale));
        }
        else if (pattern_type == 3) {
            // 체크 무늬 (Checker) - Croatia 스타일
            float x = step(0.5, fract(pixel_coord.x / pattern_scale));
            float y = step(0.5, fract(pixel_coord.y / pattern_scale));
            pattern_val = abs(x - y);
        }
        else if (pattern_type == 4) {
            // 대각선 줄무늬 (Diagonal)
            float diag = pixel_coord.x + pixel_coord.y;
            pattern_val = step(pattern_ratio, fract(diag / pattern_scale));
        }

        // 메인/서브 컬러 믹스
        vec3 kit_color = mix(primary_color.rgb, secondary_color.rgb, pattern_val);

        // 원본 명암 보존 (옵션)
        if (preserve_shading) {
            // 원본의 밝기(luminance) 계산
            float original_lum = dot(tex.rgb, vec3(0.299, 0.587, 0.114));
            float key_lum = dot(key_color.rgb, vec3(0.299, 0.587, 0.114));

            // 명암 비율 적용 (원본이 키 컬러보다 어두우면 결과도 어둡게)
            float shade_ratio = original_lum / max(key_lum, 0.01);
            shade_ratio = clamp(shade_ratio, 0.3, 1.2);  // 너무 극단적인 값 방지

            kit_color *= shade_ratio;
        }

        COLOR = vec4(kit_color, tex.a);
    }
    else {
        // === 옷이 아닌 영역 (피부, 머리 등) - 원본 유지 ===
        COLOR = tex;
    }
}
