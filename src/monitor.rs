use imlib_rs::{
    imlib_context_new, imlib_context_pop, imlib_context_push, imlib_context_set_color_range,
    imlib_context_set_colormap, imlib_context_set_display, imlib_context_set_drawable,
    imlib_context_set_visual, imlib_create_color_range, Imlib_Context,
};
use log::info;
use x11::xlib::{
    Pixmap, Window, XCreatePixmap, XDefaultColormap, XDefaultDepth, XDefaultVisual, XDisplayHeight,
    XDisplayWidth, XOpenDisplay, XRootWindow, XScreenCount, _XDisplay,
};

#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    pub root: Window,
    pub pixmap: Pixmap,
    pub width: u32,
    pub height: u32,
    pub render_context: Imlib_Context,
}

struct Cast<A, B>((A, B));
impl<A, B> Cast<A, B> {
    const ASSERT_ALIGN_GREATER_THAN_EQUAL: () = assert!(align_of::<A>() >= align_of::<B>());
    const ASSERT_SIZE_EQUAL: () = assert!(size_of::<A>() == size_of::<B>());

    fn safe_ptr_cast(a: *mut A) -> *mut B {
        let _ = Self::ASSERT_SIZE_EQUAL;
        let _ = Self::ASSERT_ALIGN_GREATER_THAN_EQUAL;

        a.cast()
    }
}

pub unsafe fn get_monitors() -> (*mut _XDisplay, Vec<Monitor>) {
    let display = XOpenDisplay(std::ptr::null());

    let screen_count = XScreenCount(display);

    info!("Found {} screens", screen_count);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen in 0..screen_count {
        info!("Running screen {}", current_screen);

        let width = XDisplayWidth(display, current_screen);
        let height = XDisplayHeight(display, current_screen);
        let depth = XDefaultDepth(display, current_screen);
        let visual = XDefaultVisual(display, current_screen);

        // Total insanity because for some reason for my second monitor it just
        // returns 0x8 and segfaults on imlib_context_set_visual
        if visual as usize == 0x8 {
            continue;
        }

        let cm = XDefaultColormap(display, current_screen);

        info!(
            "Screen {}: width: {}, height: {}, depth: {}",
            current_screen, width, height, depth
        );

        let root = XRootWindow(display, current_screen);
        let pixmap = XCreatePixmap(display, root, width as u32, height as u32, depth as u32);

        monitors.push(Monitor {
            root,
            pixmap,
            width: width as u32,
            height: height as u32,
            render_context: imlib_context_new(),
        });

        imlib_context_push(monitors[current_screen as usize].render_context);
        imlib_context_set_display(display.cast());
        imlib_context_set_visual(Cast::safe_ptr_cast(visual));
        imlib_context_set_colormap(cm);
        imlib_context_set_drawable(pixmap);
        imlib_context_set_color_range(imlib_create_color_range());
        imlib_context_pop();
    }

    info!("Loaded {} screens", screen_count);

    (display, monitors)
}
