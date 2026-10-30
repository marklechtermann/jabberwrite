//! Recording badge for Wayland sessions (ADR 0008).
//!
//! xdg-shell windows can neither be positioned nor kept on top, so the badge is
//! a `zwlr_layer_shell_v1` surface drawn in software into a shared-memory
//! buffer. It runs on its own thread with its own Wayland connection. When the
//! compositor has no layer-shell (GNOME) there is simply no overlay.

use std::time::Instant;

use smithay_client_toolkit::compositor::{
    CompositorHandler, CompositorState, FrameCallbackData, Region,
};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::channel::{self, Channel, Sender};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopSignal};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};

/// Badge edge length in pixels (surface and buffer; buffer scale is 1).
const PX: u32 = super::BADGE as u32;
const MARGIN_BOTTOM: i32 = super::BOTTOM_MARGIN as i32;
/// Seconds per ring revolution (CSS `.ring`: 1.1s linear).
const SPIN_SECS: f32 = 1.1;
/// A surface the compositor keeps closing is given up on after this many
/// attempts without a configure in between.
const MAX_CLOSES: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Recording,
    Processing,
    Error,
}

enum Command {
    Show(Kind),
    Hide,
}

/// Sender side of the layer-shell thread; stored as Tauri managed state.
pub struct Handle(Sender<Command>);

impl Handle {
    /// Show the badge in `state` ("recording", "processing" or "error").
    pub fn show(&self, state: &str) {
        let kind = match state {
            "recording" => Kind::Recording,
            "processing" => Kind::Processing,
            "error" => Kind::Error,
            other => {
                tracing::debug!("Unknown overlay state {other:?}");
                return;
            }
        };
        // The thread has already ended when the compositor offers no layer-shell.
        let _ = self.0.send(Command::Show(kind));
    }

    /// Remove the badge surface.
    pub fn hide(&self) {
        let _ = self.0.send(Command::Hide);
    }
}

/// Start the layer-shell thread. Failures (no Wayland display, no layer-shell)
/// are logged from the thread and leave the overlay absent.
pub fn start() -> Handle {
    let (tx, rx) = channel::channel();
    let spawned = std::thread::Builder::new()
        .name("overlay-layer".into())
        .spawn(move || {
            if let Err(e) = run(rx) {
                tracing::warn!("Wayland overlay disabled: {e}");
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("Wayland overlay disabled: cannot start thread: {e}");
    }
    Handle(tx)
}

fn run(rx: Channel<Command>) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("cannot connect: {e}"))?;
    let (globals, queue) =
        registry_queue_init::<Badge>(&conn).map_err(|e| format!("registry: {e}"))?;
    let qh = queue.handle();

    let compositor =
        CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?;
    let layer_shell = LayerShell::bind(&globals, &qh)
        .map_err(|e| format!("compositor has no zwlr_layer_shell_v1: {e}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?;
    let pool = SlotPool::new((PX * PX * 4) as usize, &shm).map_err(|e| format!("shm pool: {e}"))?;

    let mut event_loop: EventLoop<Badge> =
        EventLoop::try_new().map_err(|e| format!("event loop: {e}"))?;
    let handle = event_loop.handle();
    let mut badge = Badge {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        layer_shell,
        shm,
        pool,
        qh,
        signal: event_loop.get_signal(),
        kind: None,
        layer: None,
        configured: false,
        closes: 0,
        frame_pending: false,
        start: Instant::now(),
    };

    WaylandSource::new(conn, queue)
        .insert(handle.clone())
        .map_err(|e| e.to_string())?;
    handle
        .insert_source(rx, |event, _, badge| match event {
            channel::Event::Msg(cmd) => badge.command(cmd),
            channel::Event::Closed => badge.signal.stop(),
        })
        .map_err(|e| e.to_string())?;

    tracing::info!("Wayland overlay ready");
    event_loop
        .run(None, &mut badge, |_| {})
        .map_err(|e| format!("event loop: {e}"))
}

struct Badge {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    qh: QueueHandle<Badge>,
    signal: LoopSignal,
    /// What should be on screen; `None` while hidden.
    kind: Option<Kind>,
    layer: Option<LayerSurface>,
    /// The compositor has acknowledged the current surface.
    configured: bool,
    /// Consecutive `closed` events without a successful configure.
    closes: u8,
    /// A frame callback is outstanding on the current surface.
    frame_pending: bool,
    /// Origin of the ring rotation.
    start: Instant,
}

impl Badge {
    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Show(kind) => {
                self.kind = Some(kind);
                if self.layer.is_none() {
                    self.closes = 0;
                    self.create_surface();
                } else if self.configured {
                    self.draw();
                }
            }
            Command::Hide => {
                self.kind = None;
                self.configured = false;
                self.frame_pending = false;
                self.layer = None;
            }
        }
    }

    fn create_surface(&mut self) {
        let surface = self.compositor.create_surface(&self.qh);
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Overlay,
            Some("jabberwrite"),
            None,
        );
        layer.set_anchor(Anchor::BOTTOM);
        layer.set_size(PX, PX);
        layer.set_margin(0, 0, MARGIN_BOTTOM, 0);
        // Ignore other surfaces' exclusive zones (panels) when placing the badge.
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        // A fresh region is empty: clicks pass through the badge.
        match Region::new(&self.compositor) {
            Ok(region) => layer.set_input_region(Some(region.wl_region())),
            Err(e) => tracing::warn!("Overlay input region: {e}"),
        }
        // Initial commit without a buffer; the compositor answers with configure.
        layer.commit();
        self.layer = Some(layer);
        self.configured = false;
        self.frame_pending = false;
    }

    fn draw(&mut self) {
        let (Some(layer), Some(kind)) = (&self.layer, self.kind) else {
            return;
        };
        let (w, h) = (PX as i32, PX as i32);
        let (buffer, canvas) = match self
            .pool
            .create_buffer(w, h, w * 4, wl_shm::Format::Argb8888)
        {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("Overlay buffer: {e}");
                return;
            }
        };
        let turns = (self.start.elapsed().as_secs_f32() / SPIN_SECS).fract();
        render(kind, turns * std::f32::consts::TAU, canvas);
        let surface = layer.wl_surface();
        surface.damage_buffer(0, 0, w, h);
        // Only the recording ring moves; the next frame callback paces the redraw.
        if kind == Kind::Recording && !self.frame_pending {
            surface.frame(&self.qh, FrameCallbackData(surface.clone()));
            self.frame_pending = true;
        }
        if let Err(e) = buffer.attach_to(surface) {
            tracing::warn!("Overlay attach: {e}");
            return;
        }
        layer.commit();
    }
}

impl LayerShellHandler for Badge {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if self.layer.as_ref() != Some(layer) {
            return;
        }
        self.layer = None;
        self.configured = false;
        if self.kind.is_none() {
            return;
        }
        self.closes += 1;
        if self.closes <= MAX_CLOSES {
            self.create_surface();
        } else {
            tracing::warn!(
                "Compositor keeps closing the overlay surface; giving up until next show"
            );
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
        // The size is fixed; the compositor's suggestion is irrelevant.
        if self.layer.as_ref() != Some(layer) {
            return;
        }
        self.configured = true;
        self.closes = 0;
        self.draw();
    }
}

impl CompositorHandler for Badge {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        // Callbacks of an already replaced surface must not stop the loop.
        if self.layer.as_ref().map(|l| l.wl_surface()) != Some(surface) {
            return;
        }
        self.frame_pending = false;
        self.draw();
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Badge {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for Badge {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Badge {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(Badge);
delegate_dispatch2!(Badge);

// ---- software rendering ----------------------------------------------------
//
// Mirrors src/overlay/overlay.css and main.tsx: a dark disc, and per state a
// spinning red ring + white microphone, three white dots (the CSS wave is not
// mirrored), or an amber "!". All geometry is in badge pixels (56 x 56,
// centre 28, 28).

type P = (f32, f32);
/// Premultiplied RGBA, each channel 0..=1.
type Px = [f32; 4];

const CENTER: f32 = PX as f32 / 2.0;
const DISC: [f32; 3] = [24.0 / 255.0, 24.0 / 255.0, 27.0 / 255.0];
const DISC_ALPHA: f32 = 0.85;
const RED: [f32; 3] = [239.0 / 255.0, 68.0 / 255.0, 68.0 / 255.0];
const AMBER: [f32; 3] = [245.0 / 255.0, 158.0 / 255.0, 11.0 / 255.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

fn dist(a: P, b: P) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn seg_dist(p: P, a: P, b: P) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// Pixel coverage for a signed distance (negative inside), one pixel wide edge.
fn coverage(sd: f32) -> f32 {
    (0.5 - sd).clamp(0.0, 1.0)
}

fn over(dst: &mut Px, rgb: [f32; 3], alpha: f32) {
    let keep = 1.0 - alpha;
    for i in 0..3 {
        dst[i] = rgb[i] * alpha + dst[i] * keep;
    }
    dst[3] = alpha + dst[3] * keep;
}

/// Signed distance (badge pixels) of the white microphone glyph; the CSS draws
/// it in a 24 x 24 viewBox scaled to 46 % of the badge with a 1.8 round stroke.
fn mic_sd(p: P) -> f32 {
    let s = PX as f32 * 0.46 / 24.0;
    let q = ((p.0 - CENTER) / s + 12.0, (p.1 - CENTER) / s + 12.0);
    let half_stroke = 0.9;
    let capsule = seg_dist(q, (12.0, 6.0), (12.0, 11.0)) - 3.0 - half_stroke;
    let arc = if q.1 >= 11.0 {
        (dist(q, (12.0, 11.0)) - 6.5).abs()
    } else {
        dist(q, (5.5, 11.0)).min(dist(q, (18.5, 11.0)))
    } - half_stroke;
    let stem = seg_dist(q, (12.0, 17.5), (12.0, 21.0)) - half_stroke;
    let base = seg_dist(q, (8.5, 21.0), (15.5, 21.0)) - half_stroke;
    capsule.min(arc).min(stem).min(base) * s
}

/// Signed distance of the red ring segment: 30 % of the circle (radius 25,
/// stroke 2.5, round caps) starting at angle `spin`, as the CSS `.ring` draws it.
fn ring_sd(p: P, spin: f32) -> f32 {
    const RADIUS: f32 = 25.0;
    const HALF_STROKE: f32 = 1.25;
    const SPAN: f32 = 0.3 * std::f32::consts::TAU;
    let (dx, dy) = (p.0 - CENTER, p.1 - CENTER);
    let rel = (dy.atan2(dx) - spin).rem_euclid(std::f32::consts::TAU);
    if rel <= SPAN {
        (dist(p, (CENTER, CENTER)) - RADIUS).abs() - HALF_STROKE
    } else {
        let cap = |a: f32| dist(p, (CENTER + RADIUS * a.cos(), CENTER + RADIUS * a.sin()));
        cap(spin).min(cap(spin + SPAN)) - HALF_STROKE
    }
}

fn shade(kind: Kind, spin: f32, p: P) -> Px {
    let mut px: Px = [0.0; 4];
    let from_center = dist(p, (CENTER, CENTER));
    over(&mut px, DISC, DISC_ALPHA * coverage(from_center - CENTER));
    match kind {
        Kind::Recording => {
            over(&mut px, RED, coverage(ring_sd(p, spin)));
            over(&mut px, WHITE, coverage(mic_sd(p)));
        }
        Kind::Processing => {
            let d = [-11.0, 0.0, 11.0]
                .iter()
                .map(|dx| dist(p, (CENTER + dx, CENTER)) - 3.0)
                .fold(f32::MAX, f32::min);
            over(&mut px, WHITE, coverage(d));
        }
        Kind::Error => {
            let stem = seg_dist(p, (CENTER, 21.6), (CENTER, 30.4)) - 2.2;
            let dot = dist(p, (CENTER, 35.2)) - 2.4;
            over(&mut px, AMBER, coverage(stem.min(dot)));
        }
    }
    px
}

/// Render the badge as ARGB8888 (little endian, premultiplied) into `canvas`.
/// `spin` is the ring rotation in radians, clockwise.
fn render(kind: Kind, spin: f32, canvas: &mut [u8]) {
    for (i, out) in canvas.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((i as u32 % PX) as f32 + 0.5, (i as u32 / PX) as f32 + 0.5);
        let [r, g, b, a] = shade(kind, spin, (x, y)).map(|c| (c * 255.0 + 0.5) as u32);
        out.copy_from_slice(&((a << 24) | (r << 16) | (g << 8) | b).to_le_bytes());
    }
}
