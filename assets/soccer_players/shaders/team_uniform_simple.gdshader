shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

// Simple version - just recolor white areas

uniform vec3 team_color : source_color = vec3(1.0, 0.0, 0.0);
uniform float threshold : hint_range(0.5, 0.95) = 0.7;
uniform sampler2D albedo_texture : source_color;

void fragment() {
    vec4 tex = texture(albedo_texture, UV);

    // Detect white: high brightness, low color variance
    float brightness = (tex.r + tex.g + tex.b) / 3.0;
    float max_c = max(tex.r, max(tex.g, tex.b));
    float min_c = min(tex.r, min(tex.g, tex.b));
    float saturation = (max_c - min_c) / (max_c + 0.001);

    // White = bright + not saturated
    float is_white = step(threshold, brightness) * step(saturation, 0.2);

    // Smooth blend
    float mask = smoothstep(0.0, 0.3, is_white);

    // Apply team color keeping original shading
    vec3 tinted = team_color * brightness;
    vec3 final_color = mix(tex.rgb, tinted, mask);

    ALBEDO = final_color;
    ALPHA = tex.a;
}
