// Vertex shader input
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// Vertex shader output / Fragment shader input
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// Uniforms containing view-projection and model transform matrices
struct Uniforms {
    view_proj: mat4x4<f32>,
    model_transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Extract transform components from column-major matrix
    // model_transform is a 4x4 matrix where:
    // [0][0] = scale_x, [1][1] = scale_y
    // [3][0] = translate_x, [3][1] = translate_y
    let sx = uniforms.model_transform[0][0];
    let sy = uniforms.model_transform[1][1];
    let tx = uniforms.model_transform[3][0];
    let ty = uniforms.model_transform[3][1];

    // Apply transform: scale then translate
    let transformed_x = sx * in.position.x + tx;
    let transformed_y = sy * in.position.y + ty;
    let world_pos = vec4<f32>(transformed_x, transformed_y, 0.0, 1.0);

    out.clip_position = uniforms.view_proj * world_pos;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
