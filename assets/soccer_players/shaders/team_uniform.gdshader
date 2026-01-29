shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Team Colors
uniform vec3 team_primary_color : source_color = vec3(1.0, 0.0, 0.0);  // Red default
uniform vec3 team_secondary_color : source_color = vec3(0.0, 0.0, 0.0);  // Black default

// Jersey Number (optional - use with number texture)
uniform sampler2D number_texture : hint_default_white;
uniform bool show_number = false;
uniform vec2 number_position = vec2(0.5, 0.3);  // UV position for number
uniform float number_scale = 0.15;

// Detection thresholds
uniform float white_threshold : hint_range(0.5, 1.0) = 0.75;  // How white = uniform
uniform float saturation_threshold : hint_range(0.0, 0.3) = 0.15;  // Low saturation = uniform
uniform float color_intensity : hint_range(0.5, 1.5) = 1.0;

// Original texture
uniform sampler2D albedo_texture : source_color;

// Helper: RGB to HSV
vec3 rgb2hsv(vec3 c) {
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

void fragment() {
    // Sample original texture
    vec4 tex_color = texture(albedo_texture, UV);
    vec3 original_color = tex_color.rgb;

    // Convert to HSV for analysis
    vec3 hsv = rgb2hsv(original_color);
    float saturation = hsv.y;
    float value = hsv.z;  // brightness

    // Detect white/light uniform areas
    // White = high brightness + low saturation
    float is_uniform = step(saturation, saturation_threshold) * step(white_threshold, value);

    // Also catch light gray areas
    float avg_brightness = (original_color.r + original_color.g + original_color.b) / 3.0;
    float is_light = smoothstep(0.6, 0.9, avg_brightness) * (1.0 - smoothstep(0.0, saturation_threshold * 2.0, saturation));

    // Combine detection
    float uniform_mask = max(is_uniform, is_light * 0.8);

    // Apply team color while preserving shading
    // Use original brightness to maintain shadows/highlights
    vec3 tinted_color = team_primary_color * value * color_intensity;

    // Blend between original and tinted based on mask
    vec3 final_color = mix(original_color, tinted_color, uniform_mask);

    // Optional: Add jersey number
    if (show_number) {
        vec2 number_uv = (UV - number_position) / number_scale + 0.5;
        if (number_uv.x >= 0.0 && number_uv.x <= 1.0 && number_uv.y >= 0.0 && number_uv.y <= 1.0) {
            vec4 number_color = texture(number_texture, number_uv);
            final_color = mix(final_color, team_secondary_color, number_color.a);
        }
    }

    ALBEDO = final_color;
    ALPHA = tex_color.a;
}
