use imlib_rs::{
    ImlibContext, ImlibContextNew, ImlibContextPop, ImlibContextPush, ImlibContextSetColorRange,
    ImlibContextSetColormap, ImlibContextSetDisplay, ImlibContextSetDrawable,
    ImlibContextSetVisual, ImlibCreateColorRange,
};
use log::{debug, error, info, warn};
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
    pub render_context: ImlibContext,
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

    if display.is_null() {
        error!("Failed to open X Display. Exiting...");
        panic!("Cannot proceed without a valid X Display.");
    }

    let screen_count = XScreenCount(display);
    info!("Detected {} screens", screen_count);

    let mut monitors: Vec<Monitor> = Vec::with_capacity(screen_count as usize);

    for current_screen in 0..screen_count {
        debug!("Processing screen {}", current_screen);

        let width = XDisplayWidth(display, current_screen);
        let height = XDisplayHeight(display, current_screen);
        let depth = XDefaultDepth(display, current_screen);
        let visual = XDefaultVisual(display, current_screen);

        // Handle invalid visual values
        if visual as usize == 0x8 {
            warn!(
                "Screen {} has an invalid visual (0x8). Skipping this screen.",
                current_screen
            );
            continue;
        }

        let cm = XDefaultColormap(display, current_screen);

        info!(
            "Screen {}: width = {}, height = {}, depth = {}",
            current_screen, width, height, depth
        );

        let root = XRootWindow(display, current_screen);
        let pixmap = XCreatePixmap(display, root, width as u32, height as u32, depth as u32);

        if pixmap == 0 {
            error!("Failed to create pixmap for screen {}. Skipping.", current_screen);
            continue;
        }

        let render_context = ImlibContextNew();
        if render_context.is_null() {
            error!(
                "Failed to create Imlib render context for screen {}. Skipping.",
                current_screen
            );
            continue;
        }

        monitors.push(Monitor {
            root,
            pixmap,
            width: width as u32,
            height: height as u32,
            render_context,
        });

        // Set up the Imlib context for the monitor
        ImlibContextPush(render_context);
        ImlibContextSetDisplay(display.cast());
        ImlibContextSetVisual(Cast::safe_ptr_cast(visual));
        ImlibContextSetColormap(cm);
        ImlibContextSetDrawable(pixmap);
        ImlibContextSetColorRange(ImlibCreateColorRange());
        ImlibContextPop();

        debug!("Screen {} setup complete.", current_screen);
    }

    info!("Successfully initialized {} monitor(s)", monitors.len());

    (display, monitors)
}

