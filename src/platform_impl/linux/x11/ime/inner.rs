use std::collections::HashMap;
use std::collections::VecDeque;
use xim::Client;

use super::util::XimClient;
use super::{context::ImeContext, ffi};

fn build_ic_attributes(client: &impl Client, client_win: ffi::Window) -> Vec<xim::Attribute> {
    client
        .build_ic_attributes()
        .push(
            xim::AttributeName::InputStyle,
            xim::InputStyle::PREEDITNOTHING | xim::InputStyle::STATUSNOTHING,
        )
        .push(xim::AttributeName::ClientWindow, client_win as u32)
        .push(xim::AttributeName::FocusWindow, client_win as u32)
        .build()
}

#[allow(unused_variables)]
impl xim::ClientHandler<XimClient> for ImeInner {
    fn handle_connect(&mut self, client: &mut XimClient) -> Result<(), xim::ClientError> {
        client.open(b"en_US.utf-8")
    }

    fn handle_disconnect(&mut self) {}

    fn handle_open(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
    ) -> Result<(), xim::ClientError> {
        self.im = input_method_id;
        if let Some(mut ctx) = self.pending_contexts.pop() {
            ctx.im = input_method_id;
            client.create_ic(input_method_id, build_ic_attributes(client, ctx.client_win))?;
            self.create_pending_context = Some(ctx);
        }
        Ok(())
    }

    fn handle_close(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
    ) -> Result<(), xim::ClientError> {
        self.im = 0;
        Ok(())
    }

    fn handle_query_extension(
        &mut self,
        client: &mut XimClient,
        extensions: &[xim::Extension],
    ) -> Result<(), xim::ClientError> {
        Ok(())
    }

    fn handle_get_im_values(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        attributes: HashMap<xim::AttributeName, Vec<u8>>,
    ) -> Result<(), xim::ClientError> {
        Ok(())
    }

    fn handle_set_ic_values(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        input_context_id: u16,
    ) -> Result<(), xim::ClientError> {
        Ok(())
    }

    fn handle_create_ic(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        input_context_id: u16,
    ) -> Result<(), xim::ClientError> {
        let mut ctx = self
            .create_pending_context
            .take()
            .ok_or(xim::ClientError::InvalidReply)?;

        if let Some(mut ctx) = self.pending_contexts.pop() {
            ctx.im = self.im;
            client.create_ic(self.im, build_ic_attributes(client, ctx.client_win))?;
            self.create_pending_context = Some(ctx);
        }

        ctx.ic = input_context_id;
        self.context_ids.insert(ctx.client_win, input_context_id);
        self.contexts.insert(input_context_id, ctx);

        Ok(())
    }

    fn handle_destory_ic(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        input_context_id: u16,
    ) -> Result<(), xim::ClientError> {
        let ctx = self
            .contexts
            .remove(&input_context_id)
            .ok_or(xim::ClientError::InvalidReply)?;
        self.context_ids.remove(&ctx.client_win);

        Ok(())
    }

    fn handle_commit(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        input_context_id: u16,
        text: &str,
    ) -> Result<(), xim::ClientError> {
        self.contexts
            .get_mut(&input_context_id)
            .ok_or(xim::ClientError::InvalidReply)?
            .written
            .push_str(text);

        Ok(())
    }

    fn handle_forward_event(
        &mut self,
        client: &mut XimClient,
        input_method_id: u16,
        input_context_id: u16,
        flag: xim::ForwardEventFlag,
        xev: ffi::XKeyEvent,
    ) -> Result<(), xim::ClientError> {
        self.forwared_events.push_back(xev);
        Ok(())
    }

    fn handle_set_event_mask(
        &mut self,
        _client: &mut XimClient,
        _input_method_id: u16,
        input_context_id: u16,
        forward_event_mask: u32,
        synchronous_event_mask: u32,
    ) -> Result<(), xim::ClientError> {
        let ctx = self
            .contexts
            .get_mut(&input_context_id)
            .ok_or(xim::ClientError::InvalidReply)?;
        ctx.forward_event_mask = forward_event_mask as _;
        ctx.synchronous_event_mask = synchronous_event_mask as _;
        Ok(())
    }
}

pub struct ImeInner {
    pub im: u16,
    pending_contexts: Vec<ImeContext>,
    create_pending_context: Option<ImeContext>,
    pub forwared_events: VecDeque<ffi::XKeyEvent>,
    pub context_ids: HashMap<ffi::Window, u16>,
    pub contexts: HashMap<u16, ImeContext>,
    // Indicates whether or not the the input method was destroyed on the server end
    // (i.e. if ibus/fcitx/etc. was terminated/restarted)
    pub is_destroyed: bool,
    pub is_fallback: bool,
}

impl ImeInner {
    pub fn new() -> Self {
        ImeInner {
            im: 0,
            pending_contexts: Vec::new(),
            create_pending_context: None,
            forwared_events: VecDeque::new(),
            context_ids: HashMap::new(),
            contexts: HashMap::new(),
            is_destroyed: false,
            is_fallback: false,
        }
    }

    pub fn spawn_create_ic(
        &mut self,
        client: &mut XimClient,
        window: ffi::Window,
    ) -> Result<(), xim::ClientError> {
        if self.im == 0 {
            self.pending_contexts.push(ImeContext::new(0, window));
            Ok(())
        } else {
            client.create_ic(self.im, build_ic_attributes(client, window))
        }
    }

    pub fn close_im_if_necessary(
        &mut self,
        client: &mut XimClient,
    ) -> Result<bool, xim::ClientError> {
        if !self.is_destroyed {
            client.close(self.im)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn destroy_ic_if_necessary(
        &mut self,
        client: &mut XimClient,
        ic: u16,
    ) -> Result<bool, xim::ClientError> {
        if !self.is_destroyed {
            client.destory_ic(self.im, ic)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn destroy_all_contexts_if_necessary(
        &mut self,
        client: &mut XimClient,
    ) -> Result<bool, xim::ClientError> {
        if !self.is_destroyed {
            for id in self.contexts.keys() {
                client.destory_ic(self.im, *id)?;
            }
        }
        Ok(!self.is_destroyed)
    }
}
