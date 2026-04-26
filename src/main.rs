mod button;
mod color;
mod display_link;
mod div;
mod element;
mod geometry;
mod label;
mod layout;
mod metal_view;
mod renderer;
mod scene;
mod shaders;
mod style;
mod text;

use objc2_foundation::MainThreadMarker;

fn main() {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    metal_view::create_window_and_run(mtm);
}
