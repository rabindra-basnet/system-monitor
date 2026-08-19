use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::path::Path;
use std::ptr;

type GtkWidget = *mut c_void;
type GdkScreen = *mut c_void;

// Function signatures for dynamic loading
type GtkInitFn = unsafe extern "C" fn(argc: *mut c_int, argv: *mut *mut *mut c_char);
type GtkWindowNewFn = unsafe extern "C" fn(type_: c_int) -> GtkWidget;
type GtkWindowSetTitleFn = unsafe extern "C" fn(window: GtkWidget, title: *const c_char);
type GtkWindowSetDefaultSizeFn = unsafe extern "C" fn(window: GtkWidget, width: c_int, height: c_int);
type GtkWindowSetPositionFn = unsafe extern "C" fn(window: GtkWidget, position: c_int);
type GtkWindowSetIconFromFileFn = unsafe extern "C" fn(window: GtkWidget, filename: *const c_char, err: *mut *mut c_void) -> c_int;
type GtkContainerAddFn = unsafe extern "C" fn(container: GtkWidget, widget: GtkWidget);
type GtkWidgetShowAllFn = unsafe extern "C" fn(widget: GtkWidget);
type GtkMainFn = unsafe extern "C" fn();
type GtkMainQuitFn = unsafe extern "C" fn();

type GtkCssProviderNewFn = unsafe extern "C" fn() -> *mut c_void;
type GtkCssProviderLoadFromDataFn = unsafe extern "C" fn(provider: *mut c_void, data: *const c_char, length: c_long, error: *mut *mut c_void) -> c_int;
type GdkScreenGetDefaultFn = unsafe extern "C" fn() -> GdkScreen;
type GtkStyleContextAddProviderForScreenFn = unsafe extern "C" fn(screen: GdkScreen, provider: *mut c_void, priority: u32);

type VteTerminalNewFn = unsafe extern "C" fn() -> GtkWidget;
type VteTerminalSetCursorBlinkModeFn = unsafe extern "C" fn(terminal: GtkWidget, mode: c_int);
type VteTerminalSetScrollbackLinesFn = unsafe extern "C" fn(terminal: GtkWidget, lines: c_long);
type VteTerminalSetMouseAutohideFn = unsafe extern "C" fn(terminal: GtkWidget, autohide: c_int);
type VteTerminalSpawnAsyncFn = unsafe extern "C" fn(
    terminal: GtkWidget,
    pty_flags: c_int,
    working_directory: *const c_char,
    argv: *const *const c_char,
    envv: *const *const c_char,
    spawn_flags: c_int,
    child_setup: *const c_void,
    child_setup_data: *mut c_void,
    child_setup_data_destroy: *const c_void,
    timeout: c_int,
    cancellable: *mut c_void,
    callback: *const c_void,
    user_data: *mut c_void,
);

type GSignalConnectDataFn = unsafe extern "C" fn(
    instance: *mut c_void,
    detailed_signal: *const c_char,
    c_handler: unsafe extern "C" fn(*mut c_void, *mut c_void),
    data: *mut c_void,
    destroy_data: *const c_void,
    connect_flags: c_int,
) -> c_long;

type GSetPrgnameFn = unsafe extern "C" fn(prgname: *const c_char);
type GSetApplicationNameFn = unsafe extern "C" fn(app_name: *const c_char);

unsafe extern "C" fn on_window_destroy_or_child_exit(_widget: *mut c_void, _user_data: *mut c_void) {
    let gtk_lib = libc::dlopen(b"libgtk-3.so.0\0".as_ptr() as *const c_char, libc::RTLD_LAZY);
    if !gtk_lib.is_null() {
        let sym = libc::dlsym(gtk_lib, b"gtk_main_quit\0".as_ptr() as *const c_char);
        if !sym.is_null() {
            let gtk_main_quit: GtkMainQuitFn = std::mem::transmute(sym);
            gtk_main_quit();
        }
    }
}

pub fn try_launch_native_gui(forward_args: &[String]) -> Result<(), String> {
    unsafe {
        let gtk_lib = libc::dlopen(b"libgtk-3.so.0\0".as_ptr() as *const c_char, libc::RTLD_LAZY);
        if gtk_lib.is_null() {
            return Err("libgtk-3.so.0 not found on system".to_string());
        }

        let vte_lib = libc::dlopen(b"libvte-2.91.so.0\0".as_ptr() as *const c_char, libc::RTLD_LAZY);
        if vte_lib.is_null() {
            return Err("libvte-2.91.so.0 not found on system".to_string());
        }

        let glib_lib = libc::dlopen(b"libglib-2.0.so.0\0".as_ptr() as *const c_char, libc::RTLD_LAZY);
        let gobject_lib = libc::dlopen(b"libgobject-2.0.so.0\0".as_ptr() as *const c_char, libc::RTLD_LAZY);

        // Resolve symbols
        let gtk_init: GtkInitFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_init\0".as_ptr() as *const c_char));
        let gtk_window_new: GtkWindowNewFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_window_new\0".as_ptr() as *const c_char));
        let gtk_window_set_title: GtkWindowSetTitleFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_window_set_title\0".as_ptr() as *const c_char));
        let gtk_window_set_default_size: GtkWindowSetDefaultSizeFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_window_set_default_size\0".as_ptr() as *const c_char));
        let gtk_window_set_position: GtkWindowSetPositionFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_window_set_position\0".as_ptr() as *const c_char));
        let gtk_window_set_icon_from_file: GtkWindowSetIconFromFileFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_window_set_icon_from_file\0".as_ptr() as *const c_char));
        let gtk_container_add: GtkContainerAddFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_container_add\0".as_ptr() as *const c_char));
        let gtk_widget_show_all: GtkWidgetShowAllFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_widget_show_all\0".as_ptr() as *const c_char));
        let gtk_main: GtkMainFn = std::mem::transmute(libc::dlsym(gtk_lib, b"gtk_main\0".as_ptr() as *const c_char));

        let vte_terminal_new: VteTerminalNewFn = std::mem::transmute(libc::dlsym(vte_lib, b"vte_terminal_new\0".as_ptr() as *const c_char));
        let vte_terminal_set_cursor_blink_mode: VteTerminalSetCursorBlinkModeFn = std::mem::transmute(libc::dlsym(vte_lib, b"vte_terminal_set_cursor_blink_mode\0".as_ptr() as *const c_char));
        let vte_terminal_set_scrollback_lines: VteTerminalSetScrollbackLinesFn = std::mem::transmute(libc::dlsym(vte_lib, b"vte_terminal_set_scrollback_lines\0".as_ptr() as *const c_char));
        let vte_terminal_set_mouse_autohide: VteTerminalSetMouseAutohideFn = std::mem::transmute(libc::dlsym(vte_lib, b"vte_terminal_set_mouse_autohide\0".as_ptr() as *const c_char));
        let vte_terminal_spawn_async: VteTerminalSpawnAsyncFn = std::mem::transmute(libc::dlsym(vte_lib, b"vte_terminal_spawn_async\0".as_ptr() as *const c_char));

        let g_signal_connect_data: GSignalConnectDataFn = std::mem::transmute(libc::dlsym(gobject_lib, b"g_signal_connect_data\0".as_ptr() as *const c_char));

        // Set application names
        if !glib_lib.is_null() {
            let prg_sym = libc::dlsym(glib_lib, b"g_set_prgname\0".as_ptr() as *const c_char);
            if !prg_sym.is_null() {
                let g_set_prgname: GSetPrgnameFn = std::mem::transmute(prg_sym);
                g_set_prgname(b"stasis\0".as_ptr() as *const c_char);
            }
            let app_sym = libc::dlsym(glib_lib, b"g_set_application_name\0".as_ptr() as *const c_char);
            if !app_sym.is_null() {
                let g_set_application_name: GSetApplicationNameFn = std::mem::transmute(app_sym);
                g_set_application_name(b"Stasis\0".as_ptr() as *const c_char);
            }
        }

        // Initialize GTK
        let mut argc: c_int = 0;
        gtk_init(&mut argc, ptr::null_mut());

        // Create main top-level GTK window
        let window = gtk_window_new(0); // GTK_WINDOW_TOPLEVEL = 0
        if window.is_null() {
            return Err("Failed to create GtkWindow".to_string());
        }

        let title = CString::new("Stasis — System Optimizer").unwrap();
        gtk_window_set_title(window, title.as_ptr());
        gtk_window_set_default_size(window, 1180, 750);
        gtk_window_set_position(window, 1); // GTK_WIN_POS_CENTER = 1

        // Try load icon
        let icon_paths = [
            dirs_icon_path(),
            Some("/usr/share/icons/hicolor/scalable/apps/stasis.svg".to_string()),
            Some("assets/stasis.svg".to_string()),
        ];
        for icon_opt in icon_paths.into_iter().flatten() {
            if Path::new(&icon_opt).exists() {
                if let Ok(c_icon) = CString::new(icon_opt) {
                    let mut err: *mut c_void = ptr::null_mut();
                    if gtk_window_set_icon_from_file(window, c_icon.as_ptr(), &mut err) != 0 {
                        break;
                    }
                }
            }
        }

        // Set Dark Theme CSS
        let css_new_sym = libc::dlsym(gtk_lib, b"gtk_css_provider_new\0".as_ptr() as *const c_char);
        let screen_get_sym = libc::dlsym(gtk_lib, b"gdk_screen_get_default\0".as_ptr() as *const c_char);
        let add_provider_sym = libc::dlsym(gtk_lib, b"gtk_style_context_add_provider_for_screen\0".as_ptr() as *const c_char);
        let load_data_sym = libc::dlsym(gtk_lib, b"gtk_css_provider_load_from_data\0".as_ptr() as *const c_char);

        if !css_new_sym.is_null() && !screen_get_sym.is_null() && !add_provider_sym.is_null() && !load_data_sym.is_null() {
            let gtk_css_provider_new: GtkCssProviderNewFn = std::mem::transmute(css_new_sym);
            let gdk_screen_get_default: GdkScreenGetDefaultFn = std::mem::transmute(screen_get_sym);
            let gtk_style_context_add_provider_for_screen: GtkStyleContextAddProviderForScreenFn = std::mem::transmute(add_provider_sym);
            let gtk_css_provider_load_from_data: GtkCssProviderLoadFromDataFn = std::mem::transmute(load_data_sym);

            let provider = gtk_css_provider_new();
            let screen = gdk_screen_get_default();
            let css = b"window { background-color: #0d1117; } vte-terminal { background-color: #0d1117; color: #c9d1d9; }\0";
            let mut err: *mut c_void = ptr::null_mut();
            gtk_css_provider_load_from_data(provider, css.as_ptr() as *const c_char, css.len() as c_long - 1, &mut err);
            gtk_style_context_add_provider_for_screen(screen, provider, 600); // GTK_STYLE_PROVIDER_PRIORITY_APPLICATION = 600
        }

        // Create single-instance VTE Terminal widget (Zero Tabs, No '+' button)
        let terminal = vte_terminal_new();
        if terminal.is_null() {
            return Err("Failed to create VteTerminal".to_string());
        }

        vte_terminal_set_cursor_blink_mode(terminal, 1); // VTE_CURSOR_BLINK_OFF = 1
        vte_terminal_set_scrollback_lines(terminal, 0);
        vte_terminal_set_mouse_autohide(terminal, 1);

        gtk_container_add(window, terminal);

        // Connect destroy and child-exited signals
        let destroy_sig = CString::new("destroy").unwrap();
        let child_exit_sig = CString::new("child-exited").unwrap();
        g_signal_connect_data(window, destroy_sig.as_ptr(), on_window_destroy_or_child_exit, ptr::null_mut(), ptr::null(), 0);
        g_signal_connect_data(terminal, child_exit_sig.as_ptr(), on_window_destroy_or_child_exit, ptr::null_mut(), ptr::null(), 0);

        // Resolve current stasis executable path
        let current_exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("stasis"));
        let exe_str = current_exe.to_string_lossy().to_string();

        let mut child_argv_cstrings: Vec<CString> = Vec::new();
        child_argv_cstrings.push(CString::new(exe_str).unwrap());
        child_argv_cstrings.push(CString::new("-i").unwrap());

        for arg in forward_args {
            if arg != "-g" && arg != "--gui" && arg != "-i" && arg != "--inline" && arg != "--cli" {
                if let Ok(c_arg) = CString::new(arg.as_str()) {
                    child_argv_cstrings.push(c_arg);
                }
            }
        }

        let mut child_argv_ptrs: Vec<*const c_char> = child_argv_cstrings.iter().map(|s| s.as_ptr()).collect();
        child_argv_ptrs.push(ptr::null());

        let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let c_home = CString::new(home_dir).unwrap();

        // Spawn stasis inline inside the pure GTK window
        vte_terminal_spawn_async(
            terminal,
            0, // VTE_PTY_DEFAULT
            c_home.as_ptr(),
            child_argv_ptrs.as_ptr(),
            ptr::null(), // default env
            0x0008,     // G_SPAWN_SEARCH_PATH = 1 << 3
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            -1,
            ptr::null_mut(),
            ptr::null(),
            ptr::null_mut(),
        );

        gtk_widget_show_all(window);
        gtk_main();

        Ok(())
    }
}

fn dirs_icon_path() -> Option<String> {
    env::var("HOME")
        .ok()
        .map(|h| format!("{}/.local/share/icons/hicolor/scalable/apps/stasis.svg", h))
}
