use crate::{Cast, Monitor};
use log::info;

pub unsafe fn set_root_atoms(display: *mut x11::xlib::_XDisplay, monitor: Monitor) {
    let atom_root = x11::xlib::XInternAtom(
        display,
        c"_XROOTPMAP_ID".as_ptr() as *const i8,
        false as i32,
    );

    let atom_eroot = x11::xlib::XInternAtom(
        display,
        c"ESETROOT_PMAP_ID".as_ptr() as *const i8,
        false as i32,
    );

    let monitor_pixmap = monitor.pixmap as u64;

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_root,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );

    x11::xlib::XChangeProperty(
        display,
        monitor.root,
        atom_eroot,
        x11::xlib::XA_PIXMAP,
        32,
        x11::xlib::PropModeReplace,
        &monitor_pixmap as *const u64 as *const u8,
        1,
    );
}

pub unsafe fn get_monitors() -> (*mut x11::xlib::_XDisplay, Vec<Monitor>) {
    let display = x11::xlib::XOpenDisplay(std::ptr::null());

    let screen_count = x11::xlib::XScreenCount(display);

    info!("Found {} screens", screen_count);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen in 0..screen_count {
        info!("Running screen {}", current_screen);

        let width = x11::xlib::XDisplayWidth(display, current_screen);
        let height = x11::xlib::XDisplayHeight(display, current_screen);
        let depth = x11::xlib::XDefaultDepth(display, current_screen);
        let visual = x11::xlib::XDefaultVisual(display, current_screen);

        // Total insanity because for some reason for my second monitor it just
        // returns 0x8 and segfaults on imlib_context_set_visual
        if visual as usize == 0x8 {
            continue;
        }

        let cm = x11::xlib::XDefaultColormap(display, current_screen);

        info!(
            "Screen {}: width: {}, height: {}, depth: {}",
            current_screen, width, height, depth
        );

        let root = x11::xlib::XRootWindow(display, current_screen);
        let pixmap =
            x11::xlib::XCreatePixmap(display, root, width as u32, height as u32, depth as u32);

        monitors.push(Monitor {
            root,
            pixmap,
            width: width as u32,
            height: height as u32,
            render_context: imlib_rs::imlib_context_new(),
        });

        imlib_rs::imlib_context_push(monitors[current_screen as usize].render_context);
        imlib_rs::imlib_context_set_display(display.cast());
        imlib_rs::imlib_context_set_visual(Cast::safe_ptr_cast(visual));
        imlib_rs::imlib_context_set_colormap(cm);
        imlib_rs::imlib_context_set_drawable(pixmap);
        imlib_rs::imlib_context_set_color_range(imlib_rs::imlib_create_color_range());
        imlib_rs::imlib_context_pop();
    }

    info!("Loaded {} screens", screen_count);

    (display, monitors)
}
