// Surface quads: rounded rectangles, route segments, soft card shadows and glows.
// Parameters come from theme.rs through the uniform; see gpu.rs.
struct Scene {
    center: vec2<f32>, viewport: vec2<f32>, zoom: f32, dpi: f32,
    // Seconds on gpu::clock(), for highlights that fade without re-uploading geometry.
    time: f32, unused: f32,
    // offset, blur, opacity under a dark card, opacity under a white card
    shadow: vec4<f32>,
    // glow width in screen points, rise seconds, duration seconds, unused
    glow: vec4<f32>,
}
@group(0) @binding(0) var<uniform> scene: Scene;

const PLAIN: f32 = 0.;
const HALO: f32 = 2.;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) style: vec2<f32>,
    @location(5) detail: vec4<f32>,
}

fn to_clip(world: vec2<f32>) -> vec4<f32> {
    let screen = (world - scene.center) * scene.zoom + scene.viewport * 0.5;
    return vec4(screen.x / scene.viewport.x * 2. - 1., 1. - screen.y / scene.viewport.y * 2., 0., 1.);
}

// Signed distance to a rounded rectangle whose top-left corner is the origin.
fn rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(size.x, size.y) * 0.5);
    let q = abs(p - size * 0.5) - size * 0.5 + vec2(r);
    return length(max(q, vec2(0.))) + min(max(q.x, q.y), 0.) - r;
}

// Mirrors theme::changed_intensity.
fn changed_intensity(age: f32) -> f32 {
    if (age < 0. || age >= scene.glow.z) { return 0.; }
    let rise = min(age / scene.glow.y, 1.);
    let fall = 1. - max(age - scene.glow.y, 0.) / (scene.glow.z - scene.glow.y);
    return rise * fall * fall;
}

@vertex fn vertex(@builtin(vertex_index) index: u32,
                  @location(0) rect: vec4<f32>, @location(1) fill: vec4<f32>,
                  @location(2) border: vec4<f32>, @location(3) style: vec4<f32>, @location(4) detail: vec4<f32>) -> VertexOutput {
    let corners = array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    var margin = 1.5 / (scene.zoom * scene.dpi);
    if (detail.z != PLAIN) { margin += scene.glow.x / scene.zoom; }
    let local = corners[index] * (rect.zw + vec2(margin * 2.)) - vec2(margin);
    let rotated = vec2(local.x * style.z - local.y * style.w, local.x * style.w + local.y * style.z);
    var out: VertexOutput;
    out.position = to_clip(rect.xy + rotated);
    out.local = local; out.size = rect.zw; out.fill = fill; out.border = border; out.style = style.xy;
    out.detail = detail;
    return out;
}

// Soft glow around a quad: a static halo, or the changed highlight that
// rises and fades on the scene clock (ring at the edge, glow outside, tint inside).
fn glow(q: VertexOutput, distance: f32) -> vec4<f32> {
    // Screen points, so the glow reads the same at every zoom.
    let d = distance * scene.zoom;
    let outer = select(0., pow(1. - smoothstep(0., scene.glow.x, d), 2.), d > 0.);
    if (q.detail.z == HALO) { return q.fill * outer * 0.5; }
    let ring = 1. - smoothstep(1., 2.5, abs(d + 1.));
    let tint = select(0., 0.14, d < 0.);
    let strength = changed_intensity(scene.time - q.detail.w);
    return q.fill * strength * max(max(outer * 0.7, ring), tint);
}

@fragment fn fragment(q: VertexOutput) -> @location(0) vec4<f32> {
    let distance = rounded_rect(q.local, q.size, q.style.x);
    // Derivatives before any instance-dependent branch (uniform control flow).
    let aa = max(fwidth(distance), 0.001);
    if (q.detail.z != PLAIN) { return glow(q, distance); }
    let cover = 1. - smoothstep(-aa * 0.5, aa * 0.5, distance);
    let inside = 1. - smoothstep(-aa * 0.5, aa * 0.5, distance + q.style.y);
    let color = mix(q.border, q.fill, inside);
    let period = max(q.detail.x, 1.);
    let phase = q.local.x - floor(q.local.x / period) * period;
    let dash_cover = select(1., 1. - smoothstep(q.detail.y - aa * 0.5, q.detail.y + aa * 0.5, phase), q.detail.x > 0.);
    return color * cover * dash_cover;
}

// Card shadows, drawn from the card instances themselves before the cards.
// Only opaque, unrotated plain quads cast one; route segments, halos and
// translucent separators do not.
@vertex fn shadow_vertex(@builtin(vertex_index) index: u32,
                         @location(0) rect: vec4<f32>, @location(1) fill: vec4<f32>,
                         @location(2) border: vec4<f32>, @location(3) style: vec4<f32>, @location(4) detail: vec4<f32>) -> VertexOutput {
    let corners = array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    var out: VertexOutput;
    let casts = fill.a > 0.99 && abs(style.w) < 0.0001 && detail.z == PLAIN && min(rect.z, rect.w) > 4.;
    if (!casts) {
        out.position = vec4(2., 2., 0., 1.);
        return out;
    }
    let margin = scene.shadow.y + abs(scene.shadow.x);
    let local = corners[index] * (rect.zw + vec2(margin * 2.)) - vec2(margin);
    out.position = to_clip(rect.xy + local);
    out.local = local; out.size = rect.zw; out.fill = fill; out.border = border; out.style = style.xy;
    out.detail = detail;
    return out;
}

@fragment fn shadow_fragment(q: VertexOutput) -> @location(0) vec4<f32> {
    let distance = rounded_rect(q.local - vec2(0., scene.shadow.x), q.size, q.style.x);
    let blur = max(scene.shadow.y, 0.001);
    let brightness = dot(q.fill.rgb, vec3(0.2126, 0.7152, 0.0722));
    let opacity = mix(scene.shadow.z, scene.shadow.w, brightness);
    let alpha = opacity * (1. - smoothstep(-blur * 0.5, blur, distance));
    return vec4(0., 0., 0., alpha);
}
