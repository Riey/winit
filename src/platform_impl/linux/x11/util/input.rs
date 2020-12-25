use super::*;
use crate::event::ModifiersState;

pub const VIRTUAL_CORE_POINTER: c_int = 2;
pub const VIRTUAL_CORE_KEYBOARD: c_int = 3;

impl ModifiersState {
    pub(crate) fn from_x11(state: &ffi::XIModifierState) -> Self {
        ModifiersState::from_x11_mask(state.effective as c_uint)
    }

    pub(crate) fn from_x11_mask(mask: c_uint) -> Self {
        let mut m = ModifiersState::empty();
        m.set(ModifiersState::ALT, mask & ffi::Mod1Mask != 0);
        m.set(ModifiersState::SHIFT, mask & ffi::ShiftMask != 0);
        m.set(ModifiersState::CTRL, mask & ffi::ControlMask != 0);
        m.set(ModifiersState::LOGO, mask & ffi::Mod4Mask != 0);
        m
    }
}

// NOTE: Some of these fields are not used, but may be of use in the future.
pub struct PointerState<'a> {
    xconn: &'a XConnection,
    pub root: ffi::Window,
    pub child: ffi::Window,
    pub root_x: c_double,
    pub root_y: c_double,
    pub win_x: c_double,
    pub win_y: c_double,
    buttons: ffi::XIButtonState,
    modifiers: ffi::XIModifierState,
    pub group: ffi::XIGroupState,
    pub relative_to_window: bool,
}

impl<'a> PointerState<'a> {
    pub fn get_modifier_state(&self) -> ModifiersState {
        ModifiersState::from_x11(&self.modifiers)
    }
}

impl<'a> Drop for PointerState<'a> {
    fn drop(&mut self) {
        if !self.buttons.mask.is_null() {
            unsafe {
                // This is why you need to read the docs carefully...
                (self.xconn.xlib.XFree)(self.buttons.mask as _);
            }
        }
    }
}

impl XConnection {
    pub fn select_xinput_events(
        &self,
        window: c_ulong,
        device_id: c_int,
        mask: i32,
    ) -> Flusher<'_> {
        let mut event_mask = ffi::XIEventMask {
            deviceid: device_id,
            mask: &mask as *const _ as *mut c_uchar,
            mask_len: mem::size_of_val(&mask) as c_int,
        };
        unsafe {
            (self.xinput2.XISelectEvents)(
                self.display,
                window,
                &mut event_mask as *mut ffi::XIEventMask,
                1, // number of masks to read from pointer above
            );
        }
        Flusher::new(self)
    }

    #[allow(dead_code)]
    pub fn select_xkb_events(&self, device_id: c_uint, mask: c_ulong) -> Option<Flusher<'_>> {
        let status = unsafe { (self.xlib.XkbSelectEvents)(self.display, device_id, mask, mask) };
        if status == ffi::True {
            Some(Flusher::new(self))
        } else {
            None
        }
    }

    pub fn query_pointer(
        &self,
        window: ffi::Window,
        device_id: c_int,
    ) -> Result<PointerState<'_>, XError> {
        unsafe {
            let mut root = 0;
            let mut child = 0;
            let mut root_x = 0.0;
            let mut root_y = 0.0;
            let mut win_x = 0.0;
            let mut win_y = 0.0;
            let mut buttons = Default::default();
            let mut modifiers = Default::default();
            let mut group = Default::default();

            let relative_to_window = (self.xinput2.XIQueryPointer)(
                self.display,
                device_id,
                window,
                &mut root,
                &mut child,
                &mut root_x,
                &mut root_y,
                &mut win_x,
                &mut win_y,
                &mut buttons,
                &mut modifiers,
                &mut group,
            ) == ffi::True;

            self.check_errors()?;

            Ok(PointerState {
                xconn: self,
                root,
                child,
                root_x,
                root_y,
                win_x,
                win_y,
                buttons,
                modifiers,
                group,
                relative_to_window,
            })
        }
    }
}
