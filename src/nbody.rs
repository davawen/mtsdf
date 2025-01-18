use std::{f32::consts::PI, time::Duration};

use gpu::ShaderFormat;
use lsd::*;
use sdl3_sys::{events::SDL_EVENT_QUIT, gpu::{SDL_GPUVertexAttribute, SDL_GPUVertexBufferDescription, SDL_GPUVertexElementFormat, SDL_GPUVertexInputRate}, timer::SDL_GetTicksNS};

use rand::{thread_rng, Rng};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32
}

fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: Vec3
}

/// Returns the amount of time elapsed in seconds
fn get_time() -> f64 {
    let micros = unsafe { SDL_GetTicksNS()/1000 };
    (micros as f64) / 1000000.0
}

fn main() {
    let sdl = init(InitFlags::Video).unwrap();

    let window = create_window(&sdl, "My SDL Window", 800, 800, WindowFlags::Resizable).unwrap();
    let device = gpu::Device::new(ShaderFormat::Spirv, true, None).unwrap();
    device.claim_window(&window).unwrap();

    let vert = gpu::Shader::new(&device, spirv!("shaders/nbody/vert.glsl", vert), gpu::ShaderCreate {
        format: gpu::ShaderFormat::Spirv, 
        stage: gpu::ShaderStage::VERTEX,
        num_storage_buffers: 1,
        ..Default::default()
    }).unwrap();
    let frag = gpu::Shader::new(&device, spirv!("shaders/nbody/frag.glsl", frag), gpu::ShaderCreate {
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

    let compute_pipeline = gpu::ComputePipeline::new(
        &device, spirv!("shaders/nbody/comp.glsl", comp), "main", ShaderFormat::Spirv,
        0, 1, 0, 1, 0, 1, [32, 1, 1]
    ).unwrap();

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Star {
        pos: [f32; 3],
        mass: f32,
        vel: [f32; 3],
        _padding: u32
    }

    let num_stars = 640*2;

    let star_buffer1 = gpu::Buffer::<Star>::new(&device, num_stars, gpu::BufferUsage::ComputeStorageRead | gpu::BufferUsage::ComputeStorageWrite).unwrap();
    let star_buffer2 = gpu::Buffer::<Star>::new(&device, num_stars, gpu::BufferUsage::ComputeStorageRead | gpu::BufferUsage::ComputeStorageWrite).unwrap();
    let vertex_buffer = gpu::Buffer::<Vertex>::new(&device, 12, gpu::BufferUsage::Vertex | gpu::BufferUsage::ComputeStorageWrite).unwrap();

    let mut rng = thread_rng();

    {
        let cmdbuf = device.acquire_command_buffer().unwrap();

        let copy_pass = cmdbuf.begin_copy_pass();
        let mut transfer = gpu::UploadTransferBuffer::new(&device, num_stars).unwrap();
        let mut mapped = transfer.map(&device, false).unwrap();
        mapped[0] = Star {
            pos: [0.0, 0.0, 0.0], mass: 5.0, vel: [0.0, 0.0, 0.0], _padding: 0
        };

        for s in &mut mapped.slice_mut()[1..] {
            let x: f32 = rng.gen_range(-1.0..1.0);
            let y: f32 = rng.gen_range(-1.0..1.0);
            let pos = [x, y, 0.0];

            let orthox = -y;
            let orthoy = x;
            let dist = (x*x + y*y).sqrt();

            *s = Star {
                pos,
                mass: rng.gen_range(0.01..0.1),
                // vel: [rng.gen_range(-0.1..0.1), rng.gen_range(-0.1..0.1), 0.0],
                vel: [0.2 * orthox/dist, 0.2 * orthoy/dist, rng.gen_range(-0.1..0.1)],
                _padding: 0
            };
        }
        mapped.unmap();

        star_buffer1.fill_from_transfer_buffer(&copy_pass, &transfer, 0, 0);

        let circle_vertices: [Vertex; 12] = std::array::from_fn(|i| match i {
            i if i % 3 == 0 => Vertex { pos: vec3(0.0, 0.0, 0.0) },
            i if i % 3 == 1 => {
                let i = ((((i - 1)/3) as f32) / 4.0) * 2.0 * PI;
                Vertex { pos: vec3(i.cos(), i.sin(), 0.0) }
            }
            i => {
                let i = ((((i + 1)/3) as f32) / 4.0) * 2.0 * PI;
                Vertex { pos: vec3(i.cos(), i.sin(), 0.0) }
            },
        });

        vertex_buffer.fill_from_slice(&copy_pass, 0, circle_vertices.as_slice()).unwrap();

        copy_pass.end();
        let fence = cmdbuf.submit_and_acquire_fence(&device).unwrap();
        fence.wait();
    }

    let mut star_buffer_ref1 = &star_buffer1;
    let mut star_buffer_ref2 = &star_buffer2;

    let mut open = true;
    while open {
        while let Some(event) = poll_event() {
            match unsafe { event.r#type } {
                x if x == SDL_EVENT_QUIT.0 => open = false,
                _ => ()
            }
        }

        let cmdbuf = device.acquire_command_buffer().unwrap();

        cmdbuf.push_compute_uniform(0, &[ num_stars as u32 ]);

        let compute_pass = cmdbuf.begin_compute_pass(&[], &[
            star_buffer_ref2.read_write_binding(false)
        ]);

        compute_pass.bind_pipeline(&compute_pipeline);
        compute_pass.bind_buffers(0, &[star_buffer_ref1.read_binding()]);

        compute_pass.dispatch([(num_stars/32) as u32, 1, 1]);

        compute_pass.end();

        let fence = cmdbuf.submit_and_acquire_fence(&device).unwrap();
        fence.wait();

        std::mem::swap(&mut star_buffer_ref1, &mut star_buffer_ref2);

        let cmdbuf = device.acquire_command_buffer().unwrap();

        let texture = cmdbuf.acquire_swapchain_texture(&window).unwrap();

        let color_target_info = gpu::ColorTargetInfo::new_to_texture_clear(texture, lsd::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });

        let time = get_time();
        cmdbuf.push_fragment_uniform(0, &[time as f32]);

        let render_pass = cmdbuf.begin_render_pass(&[color_target_info]);

        render_pass.bind_pipeline(&render_pipeline);
        render_pass.bind_vertex_buffer(0, &[vertex_buffer.vertex_binding(0)]);
        render_pass.bind_vertex_storage_buffers(0, &[star_buffer_ref1.read_binding()]);

        render_pass.draw_primitives(12, num_stars, 0, 0);

        render_pass.end();
        cmdbuf.submit().unwrap();

        delay(Duration::from_millis(16));
    }
}
