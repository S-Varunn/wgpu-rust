use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

// The application state to maintain the overall global states (Reusable components).
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer, 
    num_indices: u32,
}

// The slider states which contain the current position and other reqired details
#[derive(Debug)]
struct Slider <'a>{
    current_position: [f32;2],
    vertices: [Vertex;8],
    indices: &'a [u16],
    clicked: bool,
    dragging: bool,
    range_ub: f64,
    rangle_lb: f64,
}
// Calculates the vertices to the slider head based on a single vertex position (Left top corner).
fn get_new_coordinates(slider_position: [f32;2]) ->[Vertex;8] {
        let curr_position_x = slider_position[0];
        let curr_position_y = slider_position[1];
        let mut result = [Vertex {
            position: [0.0,0.0,0.0],
            color: [0.5,0.0,0.5]
        };8];
        // println!("{} {}",curr_position_x,curr_position_y);
        // Head Positioning
        result[0].position[0] = curr_position_x;
        result[0].position[1] = curr_position_y;
        result[1].position[0] = curr_position_x;
        result[1].position[1] = curr_position_y-0.15;
        result[2].position[0] = curr_position_x+0.10;
        result[2].position[1] = curr_position_y-0.15;
        result[3].position[0] = curr_position_x+0.10;
        result[3].position[1] = curr_position_y;
        // Bar positioning
        result[4].position = [-0.60, 0.50, 0.0];
        result[5].position = [-0.60, 0.45, 0.0];
        result[6].position = [0.60, 0.45, 0.0];
        result[7].position = [0.60, 0.50, 0.0];

        return result;
    }


// Slider impl with required functions to access and modify state 
impl Slider<'_> {
    // Creates a new slider with default values to position the slider at a position
    // Configurable later based on layouts
    fn new() -> Self {
        // The top left position of the slider head
        let current_position = [-0.60, 0.55];
        // The vertex coordinates of the slider head and bar
        let vertices = get_new_coordinates(current_position);
        // Vertex mapping
        let indices: &[u16] = &[
            0, 1, 3,
            1, 2, 3,
            4, 5, 7,
            5, 6, 7
        ];
        // Return an instance of the slider
        Self {
            current_position,
            clicked: false,
            range_ub: 0.60,
            rangle_lb: -0.65,
            vertices: vertices,
            indices: indices,
            dragging: false,
        }
    }
    // Update the position of the slider vertices based on the current fed position
    fn update_position(&mut self, current_position: [f32;2]){
        let vertices = get_new_coordinates(current_position);
        self.vertices = vertices;
    }
    // Set true if the slider is clicked
    fn button_clicked(&mut self) {
        self.clicked = true;
    }
    // Set true if the slider is released
    fn button_released(&mut self) {
        self.clicked = false;
    }
    // To mark if the slider is being dragged
    fn dragging(&mut self){
        self.dragging = true;
    }
    // To mark that the slider has stopped dragging
    fn dragging_stop(&mut self){
        self.dragging = false;
    }
}
 
// Vertex impl
impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        }
    }
}

// App state impl
impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window, slider: &mut Slider<'_>) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        // Create an adapter with the required configurations.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        // Create a device and queue 
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();
        // Import the shader module using the macro include_wsgl!
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // The initialisation of the buffers 
        // Creating a vertex buffer based on the vertices of the component
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&slider.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Creating an index buffer based on the indices 
        let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(slider.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // The size of the indices to be fed inside the renderer 
        let num_indices = slider.indices.len() as u32;

        let surface_caps = surface.get_capabilities(&adapter);
        
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", 
                buffers: &[
                    Vertex::desc(),
                ],
            },
                fragment: Some(wgpu::FragmentState { 
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
                primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
                depth_stencil: None, 
            multisample: wgpu::MultisampleState {
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false, 
                },
            multiview: None, 
            });

        surface.configure(&device, &config);
        
        let color = wgpu::Color::BLACK;

        Self {
            surface,
            device,
            queue,
            config,
            size,
            color,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent, window: &Window, slider: &mut Slider ) -> bool {
        match event {
            // WindowEvent::CursorEntered { device_id } => {
            //     self.color = wgpu::Color::BLACK;
            //     true
            // }
            // WindowEvent::CursorLeft { device_id } => {
            //     self.color = wgpu::Color::WHITE;
            //     true
            // }
            WindowEvent::CursorMoved {  position,.. } => {
                let size = window.inner_size();
                // println!("{:?}",size);
                let x_pos = ((position.x - (size.width / 2) as f64) / size.width as f64) * 2.0 ;
                let y_pos = ((position.y - (size.height / 2) as f64) / size.height as f64) * 2.0;
                if slider.clicked == true {
                    let current_x = slider.current_position[0] as f64;
                    let current_y = slider.current_position[1] as f64;
                    if slider.dragging == false && current_x < x_pos && current_x +  0.10 > x_pos && current_y > -y_pos && current_y - 0.15 < -y_pos {
                        slider.dragging();
                    }
                    if slider.dragging == true {
                        slider.update_position([x_pos as f32,current_y as f32]);
                       
                        if x_pos >= slider.rangle_lb && x_pos <= slider.range_ub {
                            window.request_redraw();
                        }
                    }
                }
                true
            }
            WindowEvent::MouseInput {  state, button ,.. } => {
                if button == &MouseButton::Left {
                    if state == &ElementState::Pressed {
                        slider.button_clicked();
                    }
                    else if  state == &ElementState::Released && slider.clicked == true {
                        slider.button_released();
                        if slider.dragging == true {
                            //Set the current position here
                            slider.current_position = [slider.vertices[0].position[0],slider.vertices[0].position[1]];
                        }
                        slider.dragging_stop();
                    }
                }
                // println!("{:?}",slider);
                true
            }
            _ => false
        }
    }

    fn update(&mut self, slider: &mut Slider) {
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&slider.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Creating an index buffer based on the indices 
        let index_buffer = self.device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(slider.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.index_buffer = index_buffer;
        self.vertex_buffer = vertex_buffer;
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1); 
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut slider = Slider::new();

    let mut state = State::new(&window,&mut slider).await;

    event_loop.run(move |event, _, control_flow| match event {
        //We are listening for window events and if it matches we handle it
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event, &window, &mut slider) {
                match event {
            // Here we have a guard condition and check if the window is the same as our window and proceed with another match condition
            //Listening for a window event and proceeding with a guard condition
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
                //If conditon matches close the window
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                // new_inner_size is &&mut so we have to dereference it twice
                state.resize(**new_inner_size);
            }
            _ => {}
            
        }
            }
        }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            // When changes in slider is seen update the state of the slider and render 
            state.update(&mut slider);
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            // window.request_redraw();
        }
        //If window event does not match we dont do anything (match condition)
        _ => {}
    });
}
