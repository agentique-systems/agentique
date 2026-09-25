struct View { size: vec2f, zoom: f32, selected: f32, pan: vec2f, dark: f32, pad: f32 }
@group(0) @binding(0) var<uniform> view: View;
struct Vertex { @builtin(position) position: vec4f, @location(0) color: vec4f }
fn center(id: u32) -> vec2f { return vec2f(f32(id % 50u) * 240.0 + 110.0, f32(id / 50u) * 120.0 + 44.0); }
@vertex fn vs(@builtin(vertex_index) v: u32, @builtin(instance_index) instance: u32) -> Vertex {
    let corners = array<vec2f, 6>(vec2f(-1.,-1.),vec2f(1.,-1.),vec2f(1.,1.),vec2f(-1.,-1.),vec2f(1.,1.),vec2f(-1.,1.));
    let corner = corners[v]; var p: vec2f; var color: vec4f;
    if instance < 2000u {
        let source = instance % 1000u; let destination = (source + select(1u, 50u, instance >= 1000u)) % 1000u;
        let a = center(source); let b = center(destination); let d = b-a;
        p = mix(a,b,(corner.x+1.)/2.) + normalize(vec2f(-d.y,d.x))*corner.y*0.8/view.zoom;
        color = vec4f(0.24,0.32,0.39,1.);
    } else {
        let id = instance-2000u; p = center(id)+corner*vec2f(103.,38.);
        color = select(vec4f(0.13,0.20,0.25,1.),vec4f(0.25,0.62,0.69,1.),f32(id)==view.selected);
    }
    let pixel = p*view.zoom+view.pan; var output: Vertex;
    output.position=vec4f(pixel/view.size*vec2f(2.,-2.)+vec2f(-1.,1.),0.,1.); output.color=color; return output;
}
@fragment fn fs(input: Vertex) -> @location(0) vec4f { return input.color; }
