//! 校验 Rust 侧配对类型与 `protocol/fixtures` 中共享 JSON 夹具保持一致。

use qinglink_protocol::{PairingConfirm, PairingRequest, PairingTokenResponse};

#[test]
fn rust_pairing_contract_matches_shared_fixtures() {
    let request_fixture = include_str!("../../../protocol/fixtures/pairing-request.json").trim();
    let request: PairingRequest = serde_json::from_str(request_fixture).expect("pairing request");
    assert_eq!(
        serde_json::to_string(&request).expect("serialize request"),
        request_fixture
    );

    let confirm_fixture = include_str!("../../../protocol/fixtures/pairing-confirm.json").trim();
    let confirm: PairingConfirm = serde_json::from_str(confirm_fixture).expect("pairing confirm");
    assert_eq!(
        serde_json::to_string(&confirm).expect("serialize confirm"),
        confirm_fixture
    );

    let token_fixture = include_str!("../../../protocol/fixtures/pairing-token.json").trim();
    let token: PairingTokenResponse = serde_json::from_str(token_fixture).expect("pairing token");
    assert_eq!(
        serde_json::to_string(&token).expect("serialize token"),
        token_fixture
    );
}
