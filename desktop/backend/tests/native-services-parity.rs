use base64::Engine;
use nogirem_backend::{app_services, dxvk, graphics, muo};
use serde_json::Value;
#[test]
fn original_service_results() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("native-services-parity.json")).unwrap();
    for c in cases {
        let a = &c["args"];
        let actual = match c["kind"].as_str().unwrap() {
            "nvidia-checks" => graphics::nvidia_checks(a[0].as_array().unwrap(), &a[1]),
            "nvidia-normalize" => graphics::normalize_nvidia(a[0].clone()),
            "radeon" => graphics::normalize_radeon(&a[0], a[1].clone()).unwrap(),
            "dxvk-compatibility" => dxvk::compatibility(a[0].as_str().unwrap(), &a[1]),
            "notice" => app_services::normalize_notice(a[0].as_str().unwrap()).unwrap(),
            "report" => app_services::normalize_reports(&a[0]).unwrap(),
            "channel" => app_services::parse_channel(a[0].as_str().unwrap()).unwrap(),
            "muo" => muo::decode(
                &base64::engine::general_purpose::STANDARD
                    .decode(a[0].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap(),
            other => panic!("Unknown {other}"),
        };
        assert_eq!(actual, c["expected"], "case {}", c["kind"]);
    }
}
