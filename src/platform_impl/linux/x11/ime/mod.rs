mod context;
mod inner;

use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use xim::Client;

use super::{ffi, util, XConnection};

use self::{context::ImeContext, inner::ImeInner};

pub type ImeReceiver = Receiver<(ffi::Window, i16, i16)>;
pub type ImeSender = Sender<(ffi::Window, i16, i16)>;

pub struct Ime {
    client: util::XimClient,
    inner: ImeInner,
}

impl Ime {
    pub fn new(xconn: Arc<XConnection>) -> Result<Self, xim::ClientError> {
        let display = xconn.display;
        let client = unsafe { xim::xlib::XlibClient::init(xconn, display, None)? };

        Ok(Self {
            client,
            inner: ImeInner::new(),
        })
    }

    pub fn pop_forwarded(&mut self) -> Option<ffi::XKeyEvent> {
        self.inner.forwared_events.pop_front()
    }

    pub fn filter_event(&mut self, xev: &ffi::XEvent) -> Result<bool, xim::ClientError> {
        if unsafe { self.client.filter_event(xev, &mut self.inner) }? {
            return Ok(true);
        }

        let window = {
            let xev: &ffi::XAnyEvent = xev.as_ref();
            xev.window
        };

        if let Some((ctx, client)) = self.get_context_with_client(window) {
            if (xev.get_type() == ffi::KeyPress
                && (ctx.forward_event_mask & ffi::KeyPressMask != 0))
                || xev.get_type() == ffi::KeyRelease
                    && (ctx.forward_event_mask & ffi::KeyReleaseMask != 0)
            {
                let xev = unsafe { xev.key };

                client.forward_event(
                    ctx.im,
                    ctx.ic,
                    xim::ForwardEventFlag::REQUESTLOOPUPSTRING,
                    xev,
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn is_destroyed(&self) -> bool {
        self.inner.is_destroyed
    }

    pub fn create_context(&mut self, window: ffi::Window) -> Result<(), xim::ClientError> {
        self.inner.spawn_create_ic(&mut self.client, window)
    }

    pub fn get_context(&mut self, window: ffi::Window) -> Option<&mut ImeContext> {
        if self.is_destroyed() {
            return None;
        }

        self.inner
            .contexts
            .get_mut(self.inner.context_ids.get(&window)?)
    }

    pub fn get_context_with_client(
        &mut self,
        window: ffi::Window,
    ) -> Option<(&mut ImeContext, &mut util::XimClient)> {
        if self.is_destroyed() {
            return None;
        }

        Some((
            self.inner
                .contexts
                .get_mut(self.inner.context_ids.get(&window)?)?,
            &mut self.client,
        ))
    }

    pub fn remove_context(&mut self, window: ffi::Window) -> Result<bool, xim::ClientError> {
        if let Some(id) = self.inner.context_ids.remove(&window) {
            self.inner.contexts.remove(&id);
            self.inner.destroy_ic_if_necessary(&mut self.client, id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn focus(&mut self, window: ffi::Window) -> Result<bool, xim::ClientError> {
        if self.is_destroyed() {
            return Ok(false);
        }

        if let Some((context, client)) = self.get_context_with_client(window) {
            context.focus(client).map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn unfocus(&mut self, window: ffi::Window) -> Result<bool, xim::ClientError> {
        if self.is_destroyed() {
            return Ok(false);
        }
        if let Some((context, client)) = self.get_context_with_client(window) {
            context.unfocus(client).map(|_| true)
        } else {
            Ok(false)
        }
    }

    pub fn send_xim_spot(&mut self, window: ffi::Window, x: i16, y: i16) {
        if self.is_destroyed() {
            return;
        }
        if let Some((context, client)) = self.get_context_with_client(window) {
            context.set_spot(client, x, y).ok();
        }
    }
}

impl Drop for Ime {
    fn drop(&mut self) {
        let _ = self
            .inner
            .destroy_all_contexts_if_necessary(&mut self.client);
        let _ = self.inner.close_im_if_necessary(&mut self.client);
    }
}
