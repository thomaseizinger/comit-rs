use crate::{network::oneshot_protocol, SharedSwapId};
use serde::{Deserialize, Serialize};

/// The message for the finalize protocol.
#[derive(Clone, Copy, Deserialize, Debug, Serialize)]
pub struct Message {
    pub swap_id: SharedSwapId,
}

impl Message {
    pub fn new(swap_id: SharedSwapId) -> Self {
        Self { swap_id }
    }
}

impl oneshot_protocol::Message for Message {
    const INFO: &'static str = "/comit/swap/finalize/1.0.0";
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn serialization_format_stability_test() {
        let given = Message {
            swap_id: SharedSwapId::nil(),
        };

        let actual = serde_json::to_string(&given);

        assert_that(&actual)
            .is_ok_containing(r#"{"swap_id":"00000000-0000-0000-0000-000000000000"}"#.to_owned())
    }
}
