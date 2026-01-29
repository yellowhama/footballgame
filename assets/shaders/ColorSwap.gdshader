shader_type canvas_item;

// [Target] The desired result colors (Default: Red/Blue)
uniform vec4 target_hair_color : source_color = vec4(1.0, 0.0, 0.0, 1.0);
uniform vec4 target_kit_color : source_color = vec4(0.0, 0.0, 1.0, 1.0);

// [Source] The colors to look for in the original sprite
// Changed to uniform so they can be picked in the editor using the eyedropper tool
uniform vec4 source_hair_color : source_color = vec4(0.5, 0.5, 0.5, 1.0); 
uniform vec4 source_kit_color : source_color = vec4(1.0, 1.0, 1.0, 1.0);

// Threshold for color matching (0.01 is strict, 0.1 is looser)
uniform float threshold : hint_range(0.0, 1.0) = 0.1;

void fragment() {
    vec4 current_color = texture(TEXTURE, UV);
    
    // Optimization: Skip calculation for transparent pixels
    if (current_color.a == 0.0) {
        discard;
    }
    
    // Calculate distance between current pixel color and source colors
    float dist_hair = distance(current_color.rgb, source_hair_color.rgb);
    float dist_kit = distance(current_color.rgb, source_kit_color.rgb);
    
    // Apply color replacement if within threshold
    if (dist_hair < threshold) {
        // Apply target hair color, preserving original alpha
        COLOR = vec4(target_hair_color.rgb, current_color.a);
    } 
    else if (dist_kit < threshold) {
        // Apply target kit color, preserving original alpha
        COLOR = vec4(target_kit_color.rgb, current_color.a);
    } 
    else {
        // Keep original color
        COLOR = current_color;
    }
}
