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
use shiv_window::{Window, WindowClosed, WindowCreated, WindowId, WindowPlugin, Windows};
use wgpu::Surface;

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
    surface: Surface,
    window: Arc<dyn Window>,
}

impl WindowSurface {
    #[inline]
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    #[inline]
    pub fn window(&self) -> &Arc<dyn Window> {
        &self.window
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct WindowSurfaces {
    surfaces: HashMap<WindowId, WindowSurface>,
}

pub fn maintain_surface_system(
    mut created: EventReader<WindowCreated>,
    mut closed: EventReader<WindowClosed>,
    mut surfaces: ResMut<WindowSurfaces>,
    windows: Res<Windows>,
    instance: Res<wgpu::Instance>,
    adapter: Res<wgpu::Adapter>,
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

            let window_surface = WindowSurface {
                surface,
                window: window.clone(),
            };
            surfaces.insert(event.window_id, window_surface);
        }
    }

    for event in closed.iter() {
        surfaces.remove(&event.window_id);
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
        let window_surface = WindowSurface {
            surface,
            window: primary.clone(),
        };

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
