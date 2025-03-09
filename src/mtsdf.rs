use std::time::Duration;

use gpu::{BufferUsage, ShaderFormat};
use image::buffer::ConvertBuffer;
use lsd::*;
use sdl3_sys::{events::{SDL_EVENT_QUIT, SDL_EVENT_WINDOW_RESIZED}, gpu::{SDL_GPUVertexAttribute, SDL_GPUVertexBufferDescription, SDL_GPUVertexElementFormat, SDL_GPUVertexInputRate}};

pub mod sdf;

use nalgebra::{Matrix4, Vector2, Vector3, Vector4};

type Vec2 = Vector2<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;
type Mat4 = Matrix4<f32>;

const fn vec2(x: f32, y: f32) -> Vec2 { Vec2::new(x, y) }
const fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vec3::new(x, y, z) }
const fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vec4::new(x, y, z, w) }

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: Vec3
}

struct ShapeDrawer<'d> {
    shape_buffer: gpu::Buffer<'d, Vertex>,
}

fn main() {
    let font = include_bytes!("/usr/share/fonts/TTF/Iosevka-Medium.ttc").as_slice();
    let font = ttf_parser::Face::parse(font, 0).unwrap();

    // let mtsdf = sdf::generate_mtsdf(&font);

    // let mtsdf: image::RgbaImage = mtsdf.convert();
    // let rendered: image::RgbaImage = rendered.convert();

    // mtsdf.save("out.png").unwrap();
    // rendered.save("out2.png").unwrap();

    let sdl = init(InitFlags::Video).unwrap();

    let window = create_window(&sdl, "MTSDF font rendering", 800, 800, WindowFlags::Resizable).unwrap();
    let device = gpu::Device::new(ShaderFormat::Spirv, true, None).unwrap();
    device.claim_window(&window).unwrap();

    let vert = gpu::Shader::new(&device, spirv!("shaders/mtsdf/vert.glsl", vert), gpu::ShaderCreate {
        format: gpu::ShaderFormat::Spirv, 
        stage: gpu::ShaderStage::VERTEX,
        num_storage_buffers: 0,
        num_uniform_buffers: 1,
        ..Default::default()
    }).unwrap();
    let frag = gpu::Shader::new(&device, spirv!("shaders/mtsdf/frag.glsl", frag), gpu::ShaderCreate {
        format: gpu::ShaderFormat::Spirv,
        stage: gpu::ShaderStage::FRAGMENT,
        num_uniform_buffers: 1,
        ..Default::default()
    }).unwrap();

    let render_pipeline = gpu::GraphicsPipeline::new_basic(
        &device, &window, &vert, &frag,
        gpu::PrimitiveType::TRIANGLELIST, gpu::FillMode::FILL,
        &[SDL_GPUVertexBufferDescription {
            input_rate: SDL_GPUVertexInputRate::VERTEX,
            instance_step_rate: 1,
            pitch: std::mem::size_of::<Vertex>() as u32,
            slot: 0
        }],
        &[SDL_GPUVertexAttribute {
            format: SDL_GPUVertexElementFormat::FLOAT3,
            offset: 0,
            location: 0,
            buffer_slot: 0
        }]
    ).unwrap();

    let vertex_buffer: gpu::Buffer<Vertex> = gpu::Buffer::new(&device, 6, BufferUsage::Vertex).unwrap();

    {
        let cmdbuf = device.acquire_command_buffer().unwrap();
        let copy_pass = cmdbuf.begin_copy_pass();
        vertex_buffer.fill_from_slice(&copy_pass, 0, &[
            Vertex { pos: vec3(-1.0, 1.0, 0.0) },
            Vertex { pos: vec3(1.0, 1.0, 0.0) },
            Vertex { pos: vec3(1.0, -1.0, 0.0) },
            Vertex { pos: vec3(1.0, -1.0, 0.0) },
            Vertex { pos: vec3(-1.0, -1.0, 0.0) },
            Vertex { pos: vec3(-1.0, 1.0, 0.0) },
        ]).unwrap();
        copy_pass.end();
        cmdbuf.submit().unwrap();
    }

    let mut open = true;
    while open {
        while let Some(event) = poll_event() {
            match unsafe { event.r#type } {
                x if x == SDL_EVENT_QUIT.0 => open = false,
                x if x == SDL_EVENT_WINDOW_RESIZED.0 => {
                    println!("window resized!");
                },
                _ => ()
            }
        }

        let cmdbuf = device.acquire_command_buffer().unwrap();
        let texture = cmdbuf.acquire_swapchain_texture(&window).unwrap();
        let color_target_info = gpu::ColorTargetInfo::new_to_texture_clear(texture, lsd::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
        let render_pass = cmdbuf.begin_render_pass(&[color_target_info]);

        let (x, y) = lsd::mouse::get_mouse_pos();
        let (win_w, win_h) = lsd::get_window_size(&window);
        let (win_w, win_h) = (win_w as f32, win_h as f32);

        let model = Mat4::new_translation(&vec3(x, y, 0.0)) * Mat4::new_scaling(30.0);
        let view = Mat4::new_translation(&vec3(-1.0, 1.0, 0.0)) * Mat4::new_nonuniform_scaling(&vec3(2.0/win_w, -2.0/win_h, 1.0));

        #[repr(C)]
        struct Circle {
            radius: f32,
            _pad: f32,
            center: Vec2
        }

        cmdbuf.push_vertex_uniform(0, &[(model, view)]);
        cmdbuf.push_fragment_uniform(0, &[Circle { radius: 10.0, _pad: 0.0, center: vec2(x, y) }]);
        render_pass.bind_pipeline(&render_pipeline);

        render_pass.bind_vertex_buffer(0, &[vertex_buffer.vertex_binding(0)]);
        render_pass.draw_primitives(6, 1, 0, 0);

        render_pass.end();
        cmdbuf.submit().unwrap();

        delay(Duration::from_millis(16));
    }
}
