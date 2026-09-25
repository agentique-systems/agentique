struct Camera { center: vec2<f32>, viewport: vec2<f32>, zoom: f32, dpi: f32, padding: vec2<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) style: vec2<f32>,
}
@vertex fn vertex(@builtin(vertex_index) index: u32,
                  @location(0) rect: vec4<f32>, @location(1) fill: vec4<f32>,
                  @location(2) border: vec4<f32>, @location(3) style: vec4<f32>) -> VertexOutput {
    let corners = array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    let fringe = 1.5 / (camera.zoom * camera.dpi);
    let local = corners[index] * (rect.zw + vec2(fringe * 2.)) - vec2(fringe);
    let rotated = vec2(local.x * style.z - local.y * style.w, local.x * style.w + local.y * style.z);
    let screen = (rect.xy + rotated - camera.center) * camera.zoom + camera.viewport * 0.5;
    var out: VertexOutput;
    out.position = vec4(screen.x / camera.viewport.x * 2. - 1., 1. - screen.y / camera.viewport.y * 2., 0., 1.);
    out.local = local; out.size = rect.zw; out.fill = fill; out.border = border; out.style = style.xy;
    return out;
}
@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = min(in.style.x, min(in.size.x, in.size.y) * 0.5);
    let q = abs(in.local - in.size * 0.5) - in.size * 0.5 + vec2(radius);
    let distance = length(max(q,vec2(0.))) + min(max(q.x,q.y),0.) - radius;
    let aa = max(fwidth(distance),0.001);
    let cover = 1. - smoothstep(-aa * 0.5, aa * 0.5, distance);
    let inside = 1. - smoothstep(-aa * 0.5, aa * 0.5, distance + in.style.y);
    let color = mix(in.border, in.fill, inside);
    return color * cover;
}
