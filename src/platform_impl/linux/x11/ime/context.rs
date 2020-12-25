use std::os::raw::c_long;
use xim::Client;

use super::{ffi, util};

#[derive(Debug)]
pub struct ImeContext {
    pub im: u16,
    pub ic: u16,
    pub ic_spot: xim::Point,
    pub forward_event_mask: c_long,
    pub synchronous_event_mask: c_long,
    pub client_win: ffi::Window,
    pub written: String,
}

impl ImeContext {
    pub fn new(im: u16, client_win: ffi::Window) -> Self {
        Self {
            im,
            ic: 0,
            ic_spot: xim::Point { x: 0, y: 0 },
            forward_event_mask: 0,
            synchronous_event_mask: 0,
            client_win,
            written: String::new(),
        }
    }

    pub fn focus(&self, client: &mut util::XimClient) -> Result<(), xim::ClientError> {
        client.set_focus(self.im, self.ic)
    }

    pub fn unfocus(&self, client: &mut util::XimClient) -> Result<(), xim::ClientError> {
        client.unset_focus(self.im, self.ic)
    }

    pub fn set_spot(
        &mut self,
        client: &mut util::XimClient,
        x: i16,
        y: i16,
    ) -> Result<(), xim::ClientError> {
        if self.ic_spot.x == x && self.ic_spot.y == y {
            return Ok(());
        }
        self.ic_spot = xim::Point { x, y };

        let attributes = client
            .build_ic_attributes()
            .nested_list(xim::AttributeName::PreeditAttributes, |n| {
                n.push(xim::AttributeName::SpotLocation, &self.ic_spot);
            })
            .build();
        client.set_ic_values(self.im, self.ic, attributes)
    }
}
