use std::sync::Arc;

use deref_derive::{Deref, DerefMut};
use futures_lite::future;
use shiv::{
    hash_map::HashMap,
    prelude::EventReader,
    schedule::{DefaultStage, IntoSystemDescriptor, SystemLabel},
    system::{Res, ResMut},
};
use shiv_app::{App, Plugin, Plugins};
use shiv_window::{
    Window, WindowClosed, WindowCreated, WindowId, WindowPlugin, WindowResized, Windows,
};
use wgpu::{Surface, SurfaceConfiguration};

async fn init(
    instance: &wgpu::Instance,
    surface: &Surface,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to request device");

    (adapter, device, queue)
}

pub struct WindowSurface {
    pub config: SurfaceConfiguration,
    surface: Surface,
    window: Arc<dyn Window>,
}

impl WindowSurface {
    #[inline]
    pub fn new(surface: Surface, window: Arc<dyn Window>) -> Self {
        let (width, height) = window.get_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };

        Self {
            config,
            surface,
            window,
        }
    }

    #[inline]
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    #[inline]
    pub fn window(&self) -> &Arc<dyn Window> {
        &self.window
    }

    #[inline]
    pub fn configure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct WindowSurfaces {
    surfaces: HashMap<WindowId, WindowSurface>,
}

pub fn maintain_surface_system(
    mut created: EventReader<WindowCreated>,
    mut closed: EventReader<WindowClosed>,
    mut resized: EventReader<WindowResized>,
    mut surfaces: ResMut<WindowSurfaces>,
    windows: Res<Windows>,
    instance: Res<wgpu::Instance>,
    adapter: Res<wgpu::Adapter>,
    device: Res<wgpu::Device>,
) {
    for event in created.iter() {
        if surfaces.contains_key(&event.window_id) {
            continue;
        }

        if let Some(window) = windows.get(&event.window_id) {
            let surface = unsafe { instance.create_surface(&window.raw_window_handle()) };

            assert!(
                adapter.is_surface_supported(&surface),
                "Surface is not supported by the adapter, this is not great news"
            );

            let window_surface = WindowSurface::new(surface, window.clone());
            window_surface.configure(&device);
            surfaces.insert(event.window_id, window_surface);
        }
    }

    for event in closed.iter() {
        surfaces.remove(&event.window_id);
    }

    for event in resized.iter() {
        if let Some(window_surface) = surfaces.get_mut(&event.window_id) {
            window_surface.config.width = event.width;
            window_surface.config.height = event.height;
            window_surface.configure(&device);
        }
    }
}

#[derive(Clone, Copy, Debug, SystemLabel)]
pub enum WgpuSystem {
    MaintainSurface,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    fn build(&self, app: &mut App) {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        let windows = app.world.resource::<Windows>();
        let primary = windows.primary();

        let surface = unsafe { instance.create_surface(&primary.raw_window_handle()) };

        let (adapter, device, queue) = future::block_on(init(&instance, &surface));
        let window_surface = WindowSurface::new(surface, primary.clone());
        window_surface.configure(&device);

        let mut window_surfaces = WindowSurfaces::default();
        window_surfaces.insert(windows.primary_id(), window_surface);

        app.insert_resource(window_surfaces);
        app.insert_resource(instance);
        app.insert_resource(adapter);
        app.insert_resource(device);
        app.insert_resource(queue);

        app.add_system_to_stage(
            DefaultStage::First,
            maintain_surface_system.label(WgpuSystem::MaintainSurface),
        );
    }

    fn dependencies(&self, plugins: &mut Plugins) {
        plugins.add(WindowPlugin);
    }
}
