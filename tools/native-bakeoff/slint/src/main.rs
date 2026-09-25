use slint::wgpu_30::wgpu;
use slint::{ComponentHandle, ModelRc, VecModel};
#[path = "../../gpu.rs"]
mod gpu;
slint::slint! {
    import { Button, VerticalBox, HorizontalBox, ScrollView, LineEdit, StandardListView } from "std-widgets.slint";
    export component Bakeoff inherits Window {
        title: "Agentique framework bakeoff — Slint"; width:1440px; height:900px;
        in-out property <image> scene;
        in-out property <bool> dark: true;
        in-out property <int> selected:42;
        in-out property <float> zoom:0.065;
        in-out property <float> pan-x:10;
        in-out property <float> pan-y:30;
        in-out property <[StandardListViewItem]> rows;
        in-out property <bool> dialog:false;
        callback redraw();
        background: dark ? #101720 : #edf1f4;
        VerticalBox {
            HorizontalBox { height:50px;
                Text { text:"AGENTIQUE / native framework bakeoff"; color:root.dark ? white : black; font-weight:700; }
                Text { text:"System World"; color:root.dark ? #99aabb : #445566; }
                Button { text:"Theme"; clicked=>{root.dark=!root.dark;} }
                Button { text:"Project…"; clicked=>{root.dialog=true;} }
            }
            HorizontalBox {
                VerticalBox { width:220px;
                    Text { text:"PROJECT"; color:root.dark ? white : black; }
                    LineEdit { placeholder-text:"Search project"; }
                    StandardListView { model:root.rows; current-item <=> root.selected; }
                }
                Rectangle { background:#101720; clip:true;
                    Image { source:root.scene; width:parent.width; height:parent.height; image-fit:fill; }
                    TouchArea {
                        property <float> last-x;
                        property <float> last-y;
                        pointer-event(event)=>{
                            if(event.kind==PointerEventKind.down){self.last-x=self.mouse-x/1px;self.last-y=self.mouse-y/1px;}
                            if(event.kind==PointerEventKind.down && event.button==PointerEventButton.right){menu.show();}
                        }
                        clicked=>{root.selected=floor((self.mouse-y / 1px-root.pan-y)/root.zoom/120)*50+floor((self.mouse-x / 1px-root.pan-x)/root.zoom/240);root.redraw();}
                        scroll-event(event)=>{ root.zoom=max(0.03,min(3,root.zoom*(1+event.delta-y/500px)));root.redraw();return accept; }
                        moved=>{if(self.pressed){root.pan-x+=self.mouse-x/1px-self.last-x;root.pan-y+=self.mouse-y/1px-self.last-y;self.last-x=self.mouse-x/1px;self.last-y=self.mouse-y/1px;root.redraw();}}
                    }
                    menu:=PopupWindow { width:190px; height:100px; VerticalBox { Button { text:"Focus selection";clicked=>{root.zoom=1;root.pan-x=40-mod(root.selected,50)*240;root.pan-y=40-floor(root.selected/50)*120;root.redraw();menu.close();} } Button { text:"Project details…";clicked=>{root.dialog=true;menu.close();} } } }
                }
                VerticalBox { width:260px;
                    Text { text:"Subsystem " + root.selected; font-size:22px;color:root.dark ? white : black; }
                    Text { text:"Part definition";color:root.dark ? #aabbcc : #445566; }
                    Text { text:"IDENTITY\nAuthored visual fixture\nRevision: fixture-v1\n\nSTRUCTURE\n2 outgoing relationships\nAgentique / Architecture\n\nTEXT QUALITY\nÅngström · Δpressure · 模型"; color:root.dark ? #aabbcc : #445566; }
                    Rectangle { vertical-stretch:1; }
                }
            }
            Text {height:25px;text:"VISUAL FIXTURE · 1,000 nodes · 2,000 edges · Slint 1.18.1 / wgpu 30";color:root.dark ? #99aabb : #445566;}
        }
        if dialog: Rectangle { background:#27333f;x:400px;y:250px;width:500px;height:200px;border-radius:8px;VerticalBox {Text {text:"Project details — visual fixture";color:white;}LineEdit{placeholder-text:"Project name";}Button{text:"Close";clicked=>{root.dialog=false;}}} }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .require_wgpu_30(Default::default())
        .select()?;
    let app = Bakeoff::new()?;
    app.set_rows(ModelRc::new(VecModel::from(
        (0..1000)
            .map(|i| slint::StandardListViewItem::from(format!("Subsystem {i:04}").as_str()))
            .collect::<Vec<_>>(),
    )));
    let weak = app.as_weak();
    app.on_redraw(move || {
        if let Some(app) = weak.upgrade() {
            app.window().request_redraw();
        }
    });
    let weak = app.as_weak();
    let mut render: Option<(gpu::Gpu, wgpu::Texture)> = None;
    let mut frames = 0usize;
    let started = std::time::Instant::now();
    let bench = std::env::args().any(|a| a == "--bench");
    app.window().set_rendering_notifier(move |state, api| {
        let (Some(app), slint::GraphicsAPI::WGPU30 { device, queue, .. }) = (weak.upgrade(), api)
        else {
            return;
        };
        if matches!(state, slint::RenderingState::RenderingSetup) {
            println!("adapter={:?}", device.adapter_info());
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bakeoff-scene"),
                size: wgpu::Extent3d {
                    width: 850,
                    height: 760,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            app.set_scene(slint::Image::try_from(texture.clone()).unwrap());
            render = Some((
                gpu::Gpu::new(device, wgpu::TextureFormat::Rgba8Unorm),
                texture,
            ));
        }
        if matches!(state, slint::RenderingState::BeforeRendering) {
            if let Some((gpu, texture)) = &render {
                gpu.update(
                    queue,
                    gpu::View {
                        size: [850., 760.],
                        zoom: app.get_zoom(),
                        selected: app.get_selected() as f32,
                        pan: [app.get_pan_x(), app.get_pan_y()],
                        dark: 1.,
                        pad: 0.,
                    },
                );
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let view = texture.create_view(&Default::default());
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.06,
                                    g: 0.09,
                                    b: 0.12,
                                    a: 1.,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    gpu.paint(&mut pass);
                }
                queue.submit([encoder.finish()]);
            }
        }
        if matches!(state, slint::RenderingState::AfterRendering) {
            frames += 1;
            if bench {
                app.window().request_redraw();
                if frames == 360 {
                    println!(
                        "frames={} elapsed_ms={:.2} delivered_fps={:.2}",
                        frames,
                        started.elapsed().as_secs_f64() * 1000.,
                        frames as f64 / started.elapsed().as_secs_f64()
                    );
                    slint::quit_event_loop().unwrap();
                }
            }
        }
    })?;
    app.run()?;
    Ok(())
}
