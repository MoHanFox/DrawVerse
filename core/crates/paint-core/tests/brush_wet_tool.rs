use paint_core::*;
fn red() -> Pixel {
    Pixel::from_straight([1., 0., 0., 1.]).unwrap()
}
fn blue() -> Pixel {
    Pixel::from_straight([0., 0., 1., 1.]).unwrap()
}
#[test]
fn shared_tool_reserves_snapshot_and_only_commit_adopts_ink() {
    let tool = WetTool::default();
    assert!(tool.snapshot().unwrap().is_none());
    tool.clean().unwrap();
    let settings = WetMix::new(0., 0., 0., 0.5, 1., 0.).unwrap();
    tool.reload(settings, 8., red()).unwrap();
    let previous = tool.snapshot().unwrap().unwrap();
    let lease = tool.lease().unwrap();
    assert!(tool.is_busy());
    assert_eq!(
        lease.retained().unwrap().sample(0.5, 0.5),
        red().scaled(0.5)
    );
    let shared = tool.clone();
    std::thread::spawn(move || {
        assert!(matches!(shared.lease(), Err(Error::Busy)));
        assert!(matches!(shared.snapshot(), Err(Error::Busy)));
        assert!(matches!(shared.clean(), Err(Error::Busy)));
        assert!(matches!(
            shared.reload(settings, 8., blue()),
            Err(Error::Busy)
        ));
    })
    .join()
    .unwrap();
    drop(lease);
    assert!(!tool.is_busy());
    assert_eq!(
        tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
        previous.sample(0.5, 0.5)
    );
    let lease = tool.lease().unwrap();
    let next = WetCarrier::new(settings, 12., blue()).unwrap();
    lease.commit(next);
    assert!(!tool.is_busy());
    let adopted = tool.snapshot().unwrap().unwrap();
    assert_eq!(adopted.sample(0.5, 0.5), blue().scaled(0.5));
    assert_eq!(adopted.edge(), 12);
    assert_eq!(previous.sample(0.5, 0.5), red().scaled(0.5));
    for invalid in [f32::NAN, 0., 2000.25] {
        assert!(tool.reload(settings, invalid, red()).is_err());
        assert_eq!(
            tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
            adopted.sample(0.5, 0.5)
        );
    }
    tool.clean().unwrap();
    assert_eq!(
        tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
        Pixel::TRANSPARENT
    );
}
#[test]
fn real_document_cancel_and_undo_have_separate_tool_ownership() {
    let tool = WetTool::default();
    let settings = WetMix::new(0., 0., 0., 0.5, 1., 0.).unwrap();
    tool.reload(settings, 8., red()).unwrap();
    let b = BrushSettings {
        diameter: 8.,
        wet: WetSettings::new(true, false, false, false, settings),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
    .definition();
    let mut doc = Document::new(64, 64).unwrap();
    let point = InputPoint::new(32.5, 32.5, 1.);
    let lease = tool.lease().unwrap();
    doc.begin_configured_wet_brush_stroke(b.clone(), point, lease.retained())
        .unwrap();
    doc.cancel_stroke().unwrap();
    drop(lease);
    assert_eq!(doc.pixel(32, 32), Pixel::TRANSPARENT);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
        red().scaled(0.5)
    );
    let lease = tool.lease().unwrap();
    doc.begin_configured_wet_brush_stroke(b.clone(), point, lease.retained())
        .unwrap();
    let (_, carrier) = doc.end_wet_brush_stroke().unwrap();
    lease.commit(carrier);
    let adopted = tool.snapshot().unwrap().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.pixel(32, 32), Pixel::TRANSPARENT);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        adopted.revision()
    );
    doc.redo().unwrap();
    assert_eq!(doc.pixel(32, 32), red().scaled(0.5));
    let lease = tool.lease().unwrap();
    doc.begin_configured_wet_brush_stroke(b, point, lease.retained())
        .unwrap();
    assert!(doc.stroke_to(InputPoint::new(f64::NAN, 0., 1.)).is_err());
    drop(lease);
    assert!(!tool.is_busy());
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        adopted.revision()
    );
}
#[test]
fn panicking_lease_holder_releases_without_adopting_and_tools_are_independent() {
    let tool = WetTool::default();
    tool.reload(WetMix::default(), 8., red()).unwrap();
    let shared = tool.clone();
    assert!(std::thread::spawn(move || {
        let _lease = shared.lease().unwrap();
        panic!("simulated cancelled holder")
    })
    .join()
    .is_err());
    assert!(!tool.is_busy());
    assert_eq!(tool.snapshot().unwrap().unwrap().sample(0.5, 0.5), red());
    let other = WetTool::default();
    let lease = tool.lease().unwrap();
    let separate = other.lease().unwrap();
    drop(lease);
    drop(separate);
    assert!(!other.is_busy());
}
