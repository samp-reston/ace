// region: Imports

use crate::{
    dst::{MAX_TICKS, TICK_MS},
    harness::{expect_nrc, DstScenario},
};
use ace_sim::fault::FaultConfig;

// endregion: Imports

#[test]
fn request_download_not_supported_by_default() {
    let mut s = DstScenario::new(0, FaultConfig::none());

    // [0x34, data_format, addr_len_format, addr..., size...]
    s.client
        .request(&[0x34, 0x00, 0x11, 0x00, 0x01], s.runner.bus().now())
        .unwrap();
    s.tick_until_quiet(TICK_MS, MAX_TICKS);

    let nrc = expect_nrc(&mut s.client, 0x34);
    assert_eq!(
        nrc, 0x7F,
        "expected ServiceNotSupportedInActiveSession 0x7F, got 0x{:02X}",
        nrc
    );
}
