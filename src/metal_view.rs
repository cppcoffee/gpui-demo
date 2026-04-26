// The ObjC MetalView class and window setup.
// Uses objc2-app-kit typed APIs for NSApplication/NSView/NSView
// to ensure classes are properly registered via crate feature linkage.
// Falls back to raw msg_send! where the type system requires it.

use std::ffi::c_void;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use metal::foreign_types::{ForeignType, ForeignTypeRef};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send, msg_send_id, mutability,
    rc::{Allocated, Retained},
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSColor, NSColorSpace,
    NSTrackingArea, NSView,
};
use objc2_foundation::{
    CGFloat, MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use objc2_quartz_core::CAMetalLayer;

use crate::button::{Button, ButtonStyle};
use crate::color::Hsla;
use crate::display_link::{CVDisplayLink, DisplayLink};
use crate::div::Div;
use crate::element::{Element, InteractionState};
use crate::geometry::{Bounds, Point, Size};
use crate::label::Label;
use crate::layout::LayoutEngine;
use crate::renderer::MetalRenderer;
use crate::scene::{Quad, Scene};
use crate::style::Styled;

declare_class!(
    struct AppDelegate;

    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "GpuiDemoAppDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = ();
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationShouldTerminateAfterLastWindowClosed:)]
        fn should_terminate_after_last_window_closed(&self, _sender: &NSApplication) -> bool {
            true
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this: Allocated<Self> = mtm.alloc();
        unsafe { msg_send_id![super(this.set_ivars(())), init] }
    }
}

declare_class!(
    struct MetalView;

    unsafe impl ClassType for MetalView {
        type Super = NSView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MetalView";
    }

    impl DeclaredClass for MetalView {
        type Ivars = MetalViewIvars;
    }

    unsafe impl MetalView {
        #[method_id(initWithFrame:)]
        unsafe fn init_with_frame(this: Allocated<Self>, frame: NSRect) -> Option<Retained<Self>> {
            let device = metal::Device::system_default().expect("No Metal device");
            let renderer = MetalRenderer::new(&device);
            let ivars = MetalViewIvars {
                renderer: Arc::new(Mutex::new(renderer)),
                scene: Arc::new(Mutex::new(Scene::new())),
                interaction: Arc::new(Mutex::new(InteractionState::new())),
                fps_counter: Arc::new(Mutex::new(FpsCounter::new())),
                render_pending: Arc::new(AtomicBool::new(false)),
                display_link: Mutex::new(None),
                display_link_context: Mutex::new(None),
            };
            let this = this.set_ivars(ivars);
            let this: Option<Retained<Self>> = unsafe { msg_send_id![super(this), initWithFrame: frame] };
            if let Some(this) = &this {
                let _: () = unsafe { msg_send![this, setWantsLayer: true] };
                this.start_display_link();
            }
            this
        }

        #[method(wantsUpdateLayer)]
        fn wants_update_layer(&self) -> bool { true }

        #[method_id(makeBackingLayer)]
        fn make_backing_layer(&self) -> Retained<AnyObject> {
            let device = metal::Device::system_default().expect("No Metal device");
            let layer = unsafe { CAMetalLayer::new() };
            unsafe {
                let device_obj: *mut AnyObject = device.as_ptr() as *mut AnyObject;
                let _: () = msg_send![&layer, setDevice: device_obj];
                let _: () = msg_send![&layer, setOpaque: true];
                let _: () = msg_send![&layer, setContentsScale: 2.0f64];
                let count: usize = 3;
                let _: () = msg_send![&layer, setMaximumDrawableCount: count];
            }
            let ptr = Retained::into_raw(layer) as *mut AnyObject;
            unsafe { Retained::from_raw(ptr).unwrap() }
        }

        #[method(viewDidMoveToWindow)]
        unsafe fn view_did_move_to_window(&self) {
            let frame: NSRect = unsafe { msg_send![self, bounds] };
            let opts: usize = 1 | 2 | 32 | 64; // InVisibleRect, MouseMoved, ActiveAlways, EnabledDuringMouseDrag
            let tracking_class = <NSTrackingArea as ClassType>::class();
            let area: *mut AnyObject = unsafe { msg_send![tracking_class, alloc] };
            let area: *mut AnyObject = unsafe {
                msg_send![area, initWithRect: frame, options: opts, owner: self as *const Self, userInfo: std::ptr::null::<AnyObject>()]
            };
            let _: () = unsafe { msg_send![self as *const Self, addTrackingArea: area] };
        }

        #[method(mouseDown:)]
        unsafe fn mouse_down(&self, event: *mut AnyObject) {
            let ivars = self.ivars();
            let mouse_point: NSPoint = unsafe { msg_send![event, locationInWindow] };
            let frame: NSRect = unsafe { msg_send![self, frame] };
            let view_point = Point {
                x: (mouse_point.x * 2.0) as f32,
                y: ((frame.size.height - mouse_point.y) * 2.0) as f32,
            };
            ivars.interaction.lock().unwrap().mouse_position = view_point;
            ivars.interaction.lock().unwrap().update_hover();

            let hovered = ivars.interaction.lock().unwrap().hovered_id;
            if let Some(hovered_id) = hovered {
                ivars.interaction.lock().unwrap().active_id = Some(hovered_id);
                println!("Button clicked! (element_id={})", hovered_id);
            }

            unsafe { let _: () = msg_send![self as *const Self, setNeedsDisplay: true]; }
        }

        #[method(mouseUp:)]
        unsafe fn mouse_up(&self, _event: *mut AnyObject) {
            self.ivars().interaction.lock().unwrap().active_id = None;
            unsafe { let _: () = msg_send![self as *const Self, setNeedsDisplay: true]; }
        }

        #[method(mouseMoved:)]
        unsafe fn mouse_moved(&self, event: *mut AnyObject) {
            let mouse_point: NSPoint = unsafe { msg_send![event, locationInWindow] };
            let frame: NSRect = unsafe { msg_send![self, frame] };
            let view_point = Point {
                x: (mouse_point.x * 2.0) as f32,
                y: ((frame.size.height - mouse_point.y) * 2.0) as f32,
            };
            self.ivars().interaction.lock().unwrap().mouse_position = view_point;
            self.ivars().interaction.lock().unwrap().update_hover();
            unsafe { let _: () = msg_send![self as *const Self, setNeedsDisplay: true]; }
        }

        #[method(updateLayer)]
        unsafe fn update_layer(&self) {
            self.render();
        }
    }
);

struct MetalViewIvars {
    renderer: Arc<Mutex<MetalRenderer>>,
    scene: Arc<Mutex<Scene>>,
    interaction: Arc<Mutex<InteractionState>>,
    fps_counter: Arc<Mutex<FpsCounter>>,
    render_pending: Arc<AtomicBool>,
    display_link: Mutex<Option<DisplayLink>>,
    display_link_context: Mutex<Option<Box<DisplayLinkContext>>>,
}

struct DisplayLinkContext {
    view_ptr: usize,
    render_pending: Arc<AtomicBool>,
}

struct FpsCounter {
    window_start: Instant,
    frames_in_window: u32,
    fps: u32,
}

unsafe extern "C" fn display_link_callback(
    _display_link: *mut CVDisplayLink,
    _current_time: *const c_void,
    _output_time: *const c_void,
    _flags_in: i64,
    _flags_out: *mut i64,
    user_info: *mut c_void,
) -> i32 {
    let context = unsafe { &*(user_info as *const DisplayLinkContext) };

    if !context.render_pending.swap(true, Ordering::AcqRel) {
        let view_ptr = context.view_ptr;
        let render_pending = context.render_pending.clone();

        dispatch::Queue::main().exec_async(move || unsafe {
            let view = view_ptr as *const MetalView;
            (*view).render();
            render_pending.store(false, Ordering::Release);
        });
    }

    0
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            window_start: Instant::now(),
            frames_in_window: 0,
            fps: 0,
        }
    }

    fn record_frame(&mut self, now: Instant) -> u32 {
        self.frames_in_window += 1;

        let elapsed = now.duration_since(self.window_start);
        if elapsed >= Duration::from_secs(1) {
            self.fps = (self.frames_in_window as f64 / elapsed.as_secs_f64()).round() as u32;
            self.frames_in_window = 0;
            self.window_start = now;
        }

        self.fps
    }
}

pub fn create_window_and_run(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let app_delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*app_delegate)));

    let rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: 800.0,
            height: 600.0,
        },
    };

    let window_class = <objc2_app_kit::NSWindow as ClassType>::class();
    let window: *mut AnyObject = unsafe { msg_send![window_class, alloc] };
    let window: *mut AnyObject = unsafe {
        msg_send![window, initWithContentRect: rect, styleMask: 15usize, backing: 2usize, defer: false]
    };

    let title = "GPUI Demo — Zed Architecture Patterns";
    let title_ns = NSString::from_str(title);
    let _: () = unsafe { msg_send![window, setTitle: &*title_ns] };

    let view_class = <MetalView as ClassType>::class();
    let view: Allocated<MetalView> = unsafe { msg_send_id![view_class, alloc] };
    let view: Option<Retained<MetalView>> = unsafe { msg_send_id![view, initWithFrame: rect] };
    let view = view.expect("Failed to init MetalView");

    let _: () = unsafe { msg_send![window, setContentView: &*view] };
    let _: () = unsafe { msg_send![window, center] };
    let _: () = unsafe { msg_send![window, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()] };
    #[allow(deprecated)]
    {
        app.activateIgnoringOtherApps(true)
    };
    unsafe { app.run() };
}

impl MetalView {
    fn start_display_link(&self) {
        let ivars = self.ivars();
        let context = Box::new(DisplayLinkContext {
            view_ptr: self as *const Self as usize,
            render_pending: ivars.render_pending.clone(),
        });
        let context_ptr = &*context as *const DisplayLinkContext as *mut c_void;

        let mut display_link = DisplayLink::new(display_link_callback, context_ptr)
            .expect("Failed to create display link");
        display_link.start().expect("Failed to start display link");

        *ivars.display_link_context.lock().unwrap() = Some(context);
        *ivars.display_link.lock().unwrap() = Some(display_link);
    }

    fn render(&self) {
        let ivars = self.ivars();
        let frame: NSRect = unsafe { msg_send![self, frame] };
        let scale: f64 = unsafe {
            let layer: *mut AnyObject = msg_send![self, layer];
            msg_send![layer, contentsScale]
        };

        let viewport_width = (frame.size.width * scale) as f32;
        let viewport_height = (frame.size.height * scale) as f32;
        let viewport_size = Size {
            width: viewport_width,
            height: viewport_height,
        };
        let fps = ivars
            .fps_counter
            .lock()
            .unwrap()
            .record_frame(Instant::now());

        ivars.interaction.lock().unwrap().clear_frame_state();

        // Build element tree
        let mut root = Div::new()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size(viewport_width, viewport_height)
            .bg(system_window_background_color());

        let button = Button::new(1, "Zed Button")
            .button_style(ButtonStyle::Filled)
            .size(180.0, 44.0)
            .rounded(8.0);

        root = root.child(button);

        // Phase 1: request_layout
        let mut layout_engine = LayoutEngine::new();
        let root_layout_id = root.request_layout(&mut layout_engine);
        layout_engine.compute(root_layout_id, viewport_size);

        let root_bounds = Bounds {
            origin: Point { x: 0.0, y: 0.0 },
            size: viewport_size,
        };

        // Phase 2: prepaint
        let mut interaction = ivars.interaction.lock().unwrap();
        let saved_mouse = interaction.mouse_position;
        let saved_active = interaction.active_id;
        root.prepaint(root_bounds, &layout_engine, &mut interaction);
        interaction.mouse_position = saved_mouse;
        interaction.active_id = saved_active;
        interaction.update_hover();
        drop(interaction);

        // Phase 3: paint
        let mut scene = ivars.scene.lock().unwrap();
        scene.clear();
        let interaction = ivars.interaction.lock().unwrap();
        root.paint(root_bounds, &mut scene, &interaction, &layout_engine);
        Self::paint_fps_overlay(fps, &mut scene, &interaction, &mut layout_engine);

        // Phase 4: finish
        scene.finish();

        // Phase 5: render via Metal
        let layer: *mut AnyObject = unsafe { msg_send![self, layer] };
        if layer.is_null() {
            return;
        }

        let drawable: *mut AnyObject = unsafe { msg_send![layer, nextDrawable] };
        if drawable.is_null() {
            return;
        }

        let drawable =
            unsafe { metal::MetalDrawableRef::from_ptr(drawable as *mut metal::CAMetalDrawable) };

        let mut renderer = ivars.renderer.lock().unwrap();
        renderer.draw(&scene, drawable, (viewport_width, viewport_height));
    }

    fn paint_fps_overlay(
        fps: u32,
        scene: &mut Scene,
        interaction: &InteractionState,
        layout_engine: &mut LayoutEngine,
    ) {
        let background_bounds = Bounds {
            origin: Point { x: 10.0, y: 10.0 },
            size: Size {
                width: 104.0,
                height: 26.0,
            },
        };

        scene.push_quad(Quad {
            order: 0,
            bounds: background_bounds,
            background: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.03,
                a: 0.58,
            },
            border_color: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.35,
                a: 0.45,
            },
            corner_radii: crate::geometry::Corners::uniform(5.0),
            border_widths: crate::geometry::Edges::uniform(1.0),
        });

        let label_bounds = Bounds {
            origin: Point { x: 18.0, y: 14.0 },
            size: Size {
                width: 88.0,
                height: 18.0,
            },
        };

        let mut fps_label = Label::new(format!("{fps} FPS"))
            .font_size(14.0)
            .text_color(Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.92,
                a: 1.0,
            })
            .size(label_bounds.size.width, label_bounds.size.height);
        let label_layout_id = fps_label.request_layout(layout_engine);
        layout_engine.compute(label_layout_id, label_bounds.size);
        let mut computed_label_bounds = layout_engine.bounds(label_layout_id);
        computed_label_bounds.origin = label_bounds.origin;
        fps_label.paint(computed_label_bounds, scene, interaction, layout_engine);
    }
}

fn system_window_background_color() -> Hsla {
    unsafe {
        let color = NSColor::windowBackgroundColor();
        let color_space = NSColorSpace::sRGBColorSpace();
        let color = color.colorUsingColorSpace(&color_space).unwrap_or(color);

        let mut red: CGFloat = 0.0;
        let mut green: CGFloat = 0.0;
        let mut blue: CGFloat = 0.0;
        let mut alpha: CGFloat = 1.0;
        color.getRed_green_blue_alpha(&mut red, &mut green, &mut blue, &mut alpha);

        let mut hsla = Hsla::from_rgb(
            red as f32 * 255.0,
            green as f32 * 255.0,
            blue as f32 * 255.0,
        );
        hsla.a = alpha as f32;
        hsla
    }
}
