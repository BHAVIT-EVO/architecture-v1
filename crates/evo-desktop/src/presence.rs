//! Evo's presence outside its window: the menu-bar room switcher and the
//! global cycle hotkey.
//!
//! Switching between live works must be one action from anywhere (the
//! Arc-browser lesson: summoning the switcher must never mean hunting for
//! window #10). Two surfaces:
//!
//! * a **menu-bar item** (NSStatusItem) listing the works by recency —
//!   the current room marked, click to enter/switch, Leave, Open Evo;
//! * a **global hotkey** (⌘⇧E) cycling rooms by recency.
//!
//! Both push [`RoomCommand`]s into a channel the app drains in its logic
//! tick — no UI state is touched from the platform side.
//!
//! The menu item actions arrive through a small Objective-C target class
//! (the same raw-runtime pattern the capture layer uses); the hotkey uses
//! Carbon's `RegisterEventHotKey` (deprecated but the canonical global
//! hotkey mechanism on macOS, and the one every launcher uses).
//!
//! Commands do not queue behind the app's frame loop: Evo's window can be
//! hidden (close hides, never dies), and a hidden window paints no
//! frames. The actions execute the shared [`RoomRuntime`] synchronously
//! on the main thread instead — the room is desktop state, and the
//! desktop never waits for a window.

use crate::rooms::{RoomCommand, RoomRuntime, SharedRooms};

#[cfg(target_os = "macos")]
mod imp {
    use super::RoomPresence;
    use crate::rooms::{RoomCommand, RoomRuntime, SharedRooms};
    use std::os::raw::{c_char, c_void};
    use std::ptr;
    use std::sync::{Arc, Mutex};

    type Bool = i8;
    type Id = *mut Object;
    type Class = *mut objc_class;
    type Sel = *mut c_void;
    type CFIndex = isize;

    #[repr(C)]
    pub struct Object {
        _priv: [u8; 0],
    }

    #[repr(C)]
    pub struct objc_class {
        _priv: [u8; 0],
    }

    // Carbon global hotkey (HIToolbox).
    const CMD_KEY: u32 = 0x0100;
    const SHIFT_KEY: u32 = 0x0200;
    const KVK_ANSI_E: u32 = 14; // kVK_ANSI_E

    type EventHotKeyRef = *mut c_void;
    type EventHotKeyID = [u32; 2];
    type EventHandlerRef = *mut c_void;
    type EventTypeSpec = [u32; 2]; // eventClass, eventKind
    type OSStatus = i32;
    type EventRef = *mut c_void;

    const K_EVENT_CLASS_KEYBOARD: u32 = 0x6B657962; // 'keyb'
    const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
    const NO_ERR: OSStatus = 0;

    #[link(name = "objc")]
    #[link(name = "Foundation", kind = "framework")]
    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn objc_allocateClassPair(
            superclass: Class,
            name: *const c_char,
            extra_bytes: usize,
        ) -> Class;
        fn class_addMethod(cls: Class, name: Sel, imp: IMP, types: *const c_char) -> Bool;
        fn objc_registerClassPair(cls: Class);
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;

        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            hot_key_id: EventHotKeyID,
            target: *mut c_void,
            options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> OSStatus;
        fn InstallEventHandler(
            target: *mut c_void,
            handler: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> OSStatus,
            count: u32,
            types: *const EventTypeSpec,
            user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> OSStatus;
        fn GetEventParameter(
            event: EventRef,
            name: u32,
            desired_type: u32,
            actual_type: *mut u32,
            buffer_size: usize,
            actual_size: *mut usize,
            buffer: *mut c_void,
        ) -> OSStatus;
        fn GetApplicationEventTarget() -> *mut c_void;
        fn RunApplicationEventLoop() -> i32;
        fn GetCurrentEventLoop() -> *mut c_void;
        fn ReceiveNextEvent(timeout: f64, out_event: *mut EventRef) -> OSStatus;
    }

    type IMP = extern "C" fn(Id, Sel, Id);

    const SYSTEM_STATUS_BAR_SEL: &[u8] = b"systemStatusBar\0";
    const STATUS_ITEM_WITH_LENGTH_SEL: &[u8] = b"statusItemWithLength:\0";
    const BUTTON_SEL: &[u8] = b"button\0";
    const SET_TITLE_SEL: &[u8] = b"setTitle:\0";
    const SET_IMAGE_SEL: &[u8] = b"setImage:\0";
    const SET_TARGET_SEL: &[u8] = b"setTarget:\0";
    const SET_ACTION_SEL: &[u8] = b"setAction:\0";
    const SET_TAG_SEL: &[u8] = b"setTag:\0";
    const TAG_SEL: &[u8] = b"tag\0";
    const SET_MENU_SEL: &[u8] = b"setMenu:\0";
    const NEW_SEL: &[u8] = b"new\0";
    const COUNT_SEL: &[u8] = b"count\0";
    const OBJECT_AT_INDEX_SEL: &[u8] = b"objectAtIndex:\0";
    const ADD_ITEM_SEL: &[u8] = b"addItem:\0";
    const ADD_ITEM_WITH_TITLE_ACTION_KEY_EQ_SEL: &[u8] =
        b"addItemWithTitle:action:keyEquivalent:\0";
    const SEPARATOR_ITEM_SEL: &[u8] = b"separatorItem\0";
    const SET_ENABLED_SEL: &[u8] = b"setEnabled:\0";
    const SET_STATE_SEL: &[u8] = b"setState:\0";
    const TITLE_SEL: &[u8] = b"title\0";
    const STRING_WITH_UTF8_SEL: &[u8] = b"stringWithUTF8String:\0";
    const UTF8_STRING_SEL: &[u8] = b"UTF8String\0";
    const IMAGE_NAMED_SEL: &[u8] = b"imageNamed:\0";
    const SET_TEMPLATE_SEL: &[u8] = b"setTemplate:\0";
    const RETAIN_SEL: &[u8] = b"retain\0";

    /// NSVariableStatusItemLength = -1.
    const NS_VARIABLE_STATUS_ITEM_LENGTH: f64 = -1.0;
    /// NSControlStateValueOn = 1 (the checkmark for the active room).
    const NS_STATE_ON: isize = 1;

    /// Menu item tags: 0..n = enter room n; n+10 = Leave; n+11 = Open.
    const TAG_LEAVE: isize = 10_000;
    const TAG_OPEN: isize = 10_001;
    const TAG_CYCLE: isize = 10_002;

    /// The shared room state the actions execute directly (main thread,
    /// synchronous — the desktop never waits for a window to paint).
    static mut COMMAND_RUNTIME: Option<Arc<Mutex<RoomRuntime>>> = None;
    /// The egui context: a room transition the app must show (notes,
    /// strip, menu dot) also wakes its frame loop when it can.
    static mut COMMAND_CONTEXT: Option<egui::Context> = None;
    static mut MENU_MAX_ROOMS: usize = 0;

    /// Runs a command: the room directly through the shared runtime; the
    /// window-showing command through AppKit (a hidden window paints no
    /// frames, so viewport commands queued for a frame would never run).
    fn run_command(command: RoomCommand) {
        unsafe {
            if let RoomCommand::Open = command {
                show_main_window();
                // Cloned through a reference to the static: a bitwise copy
                // out of it (read_volatile of Option<Arc<_>>) skips the
                // refcount increment while its drop still decrements —
                // every command would leak one decrement until the
                // runtime was freed under the app's feet.
                let context = (*std::ptr::addr_of!(COMMAND_CONTEXT)).clone();
                if let Some(ctx) = context {
                    // When frames do run, egui's own view of visibility
                    // follows the AppKit truth.
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                return;
            }
            let runtime = (*std::ptr::addr_of!(COMMAND_RUNTIME)).clone();
            if let Some(runtime) = runtime {
                if let Ok(mut runtime) = runtime.lock() {
                    runtime.execute(command);
                }
            }
            // No repaint request from here: the room has already changed
            // on the desktop, and requesting a repaint from inside the
            // menu tracking session can dispatch an egui frame
            // reentrantly into AppKit's run loop — eframe is not
            // reentrant, and the corruption that follows is not worth a
            // marginally fresher strip. The UI syncs at the next natural
            // frame.
        }
    }

    extern "C" fn menu_action(this: Id, _cmd: Sel, sender: Id) {
        unsafe {
            let tag = msg_send_isize(sender, sel(TAG_SEL));
            let max_rooms = std::ptr::read_volatile(std::ptr::addr_of!(MENU_MAX_ROOMS));
            let command = if tag >= 0 && (tag as usize) < max_rooms {
                RoomCommand::Enter(tag as usize)
            } else if tag == TAG_LEAVE {
                RoomCommand::Leave
            } else if tag == TAG_OPEN {
                RoomCommand::Open
            } else if tag == TAG_CYCLE {
                RoomCommand::Cycle
            } else {
                return;
            };
            run_command(command);
            let _ = this;
        }
    }

    extern "C" fn hotkey_handler(
        _next: *mut c_void,
        event: EventRef,
        user_data: *mut c_void,
    ) -> OSStatus {
        unsafe {
            // kEventParamDirectObject ('----'), typeEventHotKeyID
            let mut hot_key_id: EventHotKeyID = [0u32; 2];
            let status = GetEventParameter(
                event,
                0x2D2D2D2D, // '----'
                0x68686B69, // 'hhki'
                ptr::null_mut(),
                std::mem::size_of::<EventHotKeyID>(),
                ptr::null_mut(),
                &mut hot_key_id as *mut _ as *mut c_void,
            );
            if status == NO_ERR && hot_key_id[0] == 1 {
                run_command(RoomCommand::Cycle);
            }
            let _ = user_data;
            NO_ERR
        }
    }

    /// Shows and focuses Evo's window through AppKit directly — the menu
    /// action runs on the main thread, so no dispatch is needed and no
    /// frame has to be painted first.
    unsafe fn show_main_window() {
        let app = msg_send_id(
            objc_getClass(b"NSApplication\0".as_ptr() as *const c_char) as Id,
            sel(b"sharedApplication\0"),
        );
        if app.is_null() {
            return;
        }
        let function: extern "C" fn(Id, Sel, isize) -> Bool =
            std::mem::transmute(objc_msgSend as *const ());
        // activateIgnoringOtherApps: — bring the whole app forward.
        let _ = function(app, sel(b"activateIgnoringOtherApps:\0"), 1);
        let windows = msg_send_id(app, sel(b"windows\0"));
        if windows.is_null() {
            return;
        }
        let count = msg_send_usize(windows, sel(COUNT_SEL));
        for i in 0..count {
            let window = msg_send_id1(windows, sel(OBJECT_AT_INDEX_SEL), i as Id);
            if window.is_null() {
                continue;
            }
            // makeKeyAndOrderFront: shows the hidden window.
            let _ = msg_send_id(window, sel(b"makeKeyAndOrderFront:\0"));
            return;
        }
    }

    pub struct Presence {
        _status_item: Id,
        _menu: Id,
        _target: Id,
        _hot_key: EventHotKeyRef,
    }

    impl Presence {
        pub fn new(runtime: SharedRooms, context: egui::Context) -> Option<Self> {
            unsafe {
                std::ptr::write_volatile(std::ptr::addr_of_mut!(COMMAND_RUNTIME), Some(runtime));
                std::ptr::write_volatile(std::ptr::addr_of_mut!(COMMAND_CONTEXT), Some(context));
                std::ptr::write_volatile(std::ptr::addr_of_mut!(MENU_MAX_ROOMS), 0);

                // The action target class.
                let Some(class) = ensure_target_class() else {
                    eprintln!("EVO-PRESENCE: target class allocation failed");
                    return None;
                };
                let target: Id = msg_send_id(class as Id, sel(NEW_SEL));
                if target.is_null() {
                    eprintln!("EVO-PRESENCE: target instance failed");
                    return None;
                }

                // The status item with a template image.
                let bar = msg_send_id(class_ns_status_bar() as Id, sel(SYSTEM_STATUS_BAR_SEL));
                if bar.is_null() {
                    eprintln!("EVO-PRESENCE: systemStatusBar null");
                    return None;
                }
                let item = msg_send_id_f64(
                    bar,
                    sel(STATUS_ITEM_WITH_LENGTH_SEL),
                    NS_VARIABLE_STATUS_ITEM_LENGTH,
                );
                if item.is_null() {
                    eprintln!("EVO-PRESENCE: statusItemWithLength null");
                    return None;
                }
                let button = msg_send_id(item, sel(BUTTON_SEL));
                if !button.is_null() {
                    // A text title is the simplest reliable status-item
                    // identity; template images vary by name across macOS.
                    let _ = msg_send_id2(button, sel(SET_TITLE_SEL), ns_string("Evo"));
                }

                // The menu (items rebuilt in update_rooms).
                let menu: Id = msg_send_id(class_ns_menu() as Id, sel(NEW_SEL));
                if menu.is_null() {
                    eprintln!("EVO-PRESENCE: NSMenu new null");
                    return None;
                }
                // The menu belongs to the STATUS ITEM, not its button: a
                // left-click opens it only through NSStatusItem.menu.
                let _ = msg_send_id2(item, sel(SET_MENU_SEL), menu);

                // The global hotkey: ⌘⇧E cycles rooms.
                let mut hot_key: EventHotKeyRef = ptr::null_mut();
                let status = RegisterEventHotKey(
                    KVK_ANSI_E,
                    CMD_KEY | SHIFT_KEY,
                    [1, 0],
                    GetApplicationEventTarget(),
                    0,
                    &mut hot_key,
                );
                if status == NO_ERR {
                    let event_type: EventTypeSpec =
                        [K_EVENT_CLASS_KEYBOARD, K_EVENT_HOT_KEY_PRESSED];
                    let mut handler: EventHandlerRef = ptr::null_mut();
                    let _ = InstallEventHandler(
                        GetApplicationEventTarget(),
                        hotkey_handler,
                        1,
                        &event_type,
                        ptr::null_mut(),
                        &mut handler,
                    );
                }

                // Raw objc msgSend returns unretained references; AppKit
                // does NOT own the status item (we do), so an explicit
                // retain keeps it alive as long as the Presence struct.
                let _ = msg_send_id(item, sel(RETAIN_SEL));
                let _ = msg_send_id(menu, sel(RETAIN_SEL));
                let _ = msg_send_id(target, sel(RETAIN_SEL));
                eprintln!("EVO-PRESENCE: menu bar item + hotkey installed");
                Some(Presence {
                    _status_item: item,
                    _menu: menu,
                    _target: target,
                    _hot_key: hot_key,
                })
            }
        }

        /// Rebuilds the menu: rooms by recency (the active one checked),
        /// Leave, Open. Called from the app's logic tick.
        pub fn update_rooms(&mut self, rooms: &[String], active: Option<String>) {
            unsafe {
                // Clear the menu by removing all items.
                loop {
                    let count = msg_send_isize(self._menu, sel(NUMBER_OF_ITEMS_SEL));
                    if count == 0 {
                        break;
                    }
                    let _ = msg_send_isize1(self._menu, sel(REMOVE_ITEM_AT_INDEX_SEL), count - 1);
                }
                std::ptr::write_volatile(std::ptr::addr_of_mut!(MENU_MAX_ROOMS), rooms.len());

                let mut added_any = false;
                for (index, room) in rooms.iter().take(8).enumerate() {
                    let title = if Some(room) == active.as_ref() {
                        format!("● {room}")
                    } else {
                        format!("  {room}")
                    };
                    let item = msg_send_id4(
                        self._menu,
                        sel(ADD_ITEM_WITH_TITLE_ACTION_KEY_EQ_SEL),
                        ns_string(&title),
                        sel(MENU_ACTION_SEL) as Id,
                        ns_string(""),
                    );
                    if item.is_null() {
                        continue;
                    }
                    let _ = msg_send_id2(item, sel(SET_TAG_SEL), (index as isize) as Id);
                    let _ = msg_send_id2(item, sel(SET_TARGET_SEL), self._target);
                    added_any = true;
                }
                if added_any {
                    let separator: Id =
                        msg_send_id(class_ns_menu_item() as Id, sel(SEPARATOR_ITEM_SEL));
                    let _ = msg_send_id1(self._menu, sel(ADD_ITEM_SEL), separator);
                }
                let add_simple = |title: &str, tag: isize| unsafe {
                    let item = msg_send_id4(
                        self._menu,
                        sel(ADD_ITEM_WITH_TITLE_ACTION_KEY_EQ_SEL),
                        ns_string(title),
                        sel(MENU_ACTION_SEL) as Id,
                        ns_string(""),
                    );
                    if !item.is_null() {
                        let _ = msg_send_id2(item, sel(SET_TAG_SEL), tag as Id);
                        let _ = msg_send_id2(item, sel(SET_TARGET_SEL), self._target);
                    }
                };
                if active.is_some() {
                    add_simple("Leave room", TAG_LEAVE);
                }
                add_simple("Next room  ⌘⇧E", TAG_CYCLE);
                add_simple("Open Evo", TAG_OPEN);
            }
        }
    }

    unsafe fn ensure_target_class() -> Option<Class> {
        let name = b"EvoPresenceTarget\0";
        let existing = objc_getClass(name.as_ptr() as *const c_char);
        if !existing.is_null() {
            return Some(existing);
        }
        let superclass = objc_getClass(b"NSObject\0".as_ptr() as *const c_char);
        if superclass.is_null() {
            return None;
        }
        let cls = objc_allocateClassPair(superclass, name.as_ptr() as *const c_char, 0);
        if cls.is_null() {
            return None;
        }
        // - (void)menuAction:(id)sender — void return, one id arg.
        let types = b"v@:\0";
        if class_addMethod(
            cls,
            sel(MENU_ACTION_SEL),
            menu_action,
            types.as_ptr() as *const c_char,
        ) == 0
        {
            return None;
        }
        objc_registerClassPair(cls);
        Some(cls)
    }

    unsafe fn class_ns_status_bar() -> Class {
        objc_getClass(b"NSStatusBar\0".as_ptr() as *const c_char)
    }
    unsafe fn class_ns_menu() -> Class {
        objc_getClass(b"NSMenu\0".as_ptr() as *const c_char)
    }
    unsafe fn class_ns_menu_item() -> Class {
        objc_getClass(b"NSMenuItem\0".as_ptr() as *const c_char)
    }
    unsafe fn class_ns_image() -> Class {
        objc_getClass(b"NSImage\0".as_ptr() as *const c_char)
    }

    unsafe fn ns_string(value: &str) -> Id {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        msg_send_id1(
            objc_getClass(b"NSString\0".as_ptr() as *const c_char) as Id,
            sel(STRING_WITH_UTF8_SEL),
            bytes.as_ptr() as *mut c_char as Id,
        )
    }

    const NUMBER_OF_ITEMS_SEL: &[u8] = b"numberOfItems\0";
    const REMOVE_ITEM_AT_INDEX_SEL: &[u8] = b"removeItemAtIndex:\0";
    const MENU_ACTION_SEL: &[u8] = b"menuAction:\0";

    fn sel(bytes: &'static [u8]) -> Sel {
        unsafe { sel_registerName(bytes.as_ptr() as *const c_char) }
    }

    unsafe fn msg_send_id(receiver: Id, selector: Sel) -> Id {
        let function: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }
    unsafe fn msg_send_id1(receiver: Id, selector: Sel, arg: Id) -> Id {
        let function: extern "C" fn(Id, Sel, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }
    unsafe fn msg_send_id2(receiver: Id, selector: Sel, arg: Id) -> Id {
        msg_send_id1(receiver, selector, arg)
    }
    unsafe fn msg_send_id3(receiver: Id, selector: Sel, a: Id, b: Id) -> Id {
        let function: extern "C" fn(Id, Sel, Id, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, a, b)
    }
    unsafe fn msg_send_id_f64(receiver: Id, selector: Sel, arg: f64) -> Id {
        let function: extern "C" fn(Id, Sel, f64) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }
    unsafe fn msg_send_bool1(receiver: Id, selector: Sel, arg: isize) -> Bool {
        let function: extern "C" fn(Id, Sel, isize) -> Bool =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }
    unsafe fn msg_send_id4(receiver: Id, selector: Sel, a: Id, b: Id, c: Id) -> Id {
        let function: extern "C" fn(Id, Sel, Id, Id, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, a, b, c)
    }
    unsafe fn msg_send_isize1(receiver: Id, selector: Sel, arg: isize) -> isize {
        let function: extern "C" fn(Id, Sel, isize) -> isize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector, arg)
    }
    unsafe fn msg_send_usize(receiver: Id, selector: Sel) -> usize {
        let function: extern "C" fn(Id, Sel) -> usize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }
    unsafe fn msg_send_isize(receiver: Id, selector: Sel) -> isize {
        let function: extern "C" fn(Id, Sel) -> isize =
            std::mem::transmute(objc_msgSend as *const ());
        function(receiver, selector)
    }
}

/// The platform presence: menu-bar item + global hotkey. None where
/// unsupported (non-macOS), reported honestly.
pub struct RoomPresence {
    #[cfg(target_os = "macos")]
    inner: Option<imp::Presence>,
}

impl RoomPresence {
    pub fn new(runtime: SharedRooms, context: egui::Context) -> Self {
        Self {
            #[cfg(target_os = "macos")]
            inner: imp::Presence::new(runtime, context),
        }
    }

    pub fn update_rooms(&mut self, rooms: &[String], active: Option<String>) {
        #[cfg(target_os = "macos")]
        if let Some(inner) = &mut self.inner {
            inner.update_rooms(rooms, active);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (rooms, active);
        }
    }
}
