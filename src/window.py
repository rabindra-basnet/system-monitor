#!/usr/bin/env python3
import os
import sys
import gi
gi.require_version('Gtk', '3.0')
gi.require_version('Vte', '2.91')
from gi.repository import Gtk, Vte, GLib, Gdk, Pango

# Set application identity cleanly without deprecated set_wmclass
GLib.set_prgname("stasis")
GLib.set_application_name("Stasis")

class StasisAppWindow(Gtk.Window):
    def __init__(self, forward_args):
        super().__init__(title="Stasis — System Optimizer")
        self.set_role("stasis-main-window")
        self.set_default_size(1180, 750)
        self.set_position(Gtk.WindowPosition.CENTER)

        # Set icon
        icon_paths = [
            os.path.expanduser("~/.local/share/icons/hicolor/scalable/apps/stasis.svg"),
            "/usr/share/icons/hicolor/scalable/apps/stasis.svg",
            os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets/stasis.svg")
        ]
        for icon_p in icon_paths:
            if os.path.exists(icon_p):
                try:
                    self.set_icon_from_file(icon_p)
                    break
                except Exception:
                    pass

        # Dark theme styling
        css_provider = Gtk.CssProvider()
        css = b"""
        window {
            background-color: #0d1117;
        }
        vte-terminal {
            background-color: #0d1117;
            color: #c9d1d9;
        }
        """
        css_provider.load_from_data(css)
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        # Create single-instance VTE Terminal widget (NO tabs, NO + buttons)
        self.terminal = Vte.Terminal()
        self.terminal.set_cursor_blink_mode(Vte.CursorBlinkMode.OFF)
        self.terminal.set_scrollback_lines(0)
        self.terminal.set_mouse_autohide(True)

        # Set monospace font
        font_desc = Pango.FontDescription.from_string("Monospace 11")
        self.terminal.set_font(font_desc)

        # Set palette colors matching theme
        bg_color = Gdk.RGBA(13/255.0, 17/255.0, 23/255.0, 1.0)
        fg_color = Gdk.RGBA(201/255.0, 209/255.0, 217/255.0, 1.0)
        self.terminal.set_colors(fg_color, bg_color, [])

        self.add(self.terminal)

        # Resolve stasis binary
        stasis_bin = os.path.expanduser("~/.local/bin/stasis")
        if not os.path.exists(stasis_bin):
            stasis_bin = "/usr/local/bin/stasis"
        if not os.path.exists(stasis_bin):
            stasis_bin = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "target/release/stasis")

        # Filter out gui flags so child runs directly inline
        filtered_args = [a for a in forward_args if a not in ("-g", "--gui", "-i", "--inline", "--cli")]
        argv = [stasis_bin, "-i"] + filtered_args

        # Spawn stasis inside the native window
        try:
            self.terminal.spawn_async(
                Vte.PtyFlags.DEFAULT,
                os.path.expanduser("~"),
                argv,
                [],
                GLib.SpawnFlags.SEARCH_PATH,
                None,
                None,
                -1,
                None,
                self.on_spawn_completed,
                None
            )
        except Exception as e:
            # Fallback spawn_sync
            self.terminal.spawn_sync(
                Vte.PtyFlags.DEFAULT,
                os.path.expanduser("~"),
                argv,
                [],
                GLib.SpawnFlags.SEARCH_PATH,
                None,
                None,
                None
            )

        self.terminal.connect("child-exited", self.on_child_exited)
        self.connect("destroy", Gtk.main_quit)

    def on_spawn_completed(self, terminal, pid, error, user_data):
        if error:
            print(f"Failed to spawn stasis: {error}", file=sys.stderr)
            Gtk.main_quit()

    def on_child_exited(self, terminal, status):
        Gtk.main_quit()

if __name__ == "__main__":
    args = sys.argv[1:]
    app = StasisAppWindow(args)
    app.show_all()
    Gtk.main()
