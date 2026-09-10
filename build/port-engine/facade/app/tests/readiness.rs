use port_engine_app::w0_ready;

#[test]
fn driver_readiness_is_claimed() {
    assert!(w0_ready());
}
