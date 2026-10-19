use nogirem_backend::{settings, topology};
use serde_json::{Value, json};
#[test]
fn original_backend_parity() {
    let cases: Value = serde_json::from_str(include_str!("legacy-parity.json")).unwrap();
    for case in cases["blackbox"].as_array().unwrap() {
        let input = &case["input"];
        assert_eq!(
            settings::blackbox(input),
            case["normalized"],
            "blackbox input: {input}"
        );
        assert_eq!(json!(settings::bitrate(input)), case["bitrate"]);
        assert_eq!(
            json!(settings::quality(input, 8, 8 * 1024 * 1024 * 1024)),
            case["low"]
        );
        assert_eq!(
            json!(settings::quality(input, 16, 32 * 1024 * 1024 * 1024)),
            case["high"]
        );
    }
    for case in cases["topology"].as_array().unwrap() {
        let cpus: Vec<topology::CpuSet> = serde_json::from_value(case["cpuSets"].clone()).unwrap();
        let actual = topology::build_fixture(
            &cpus,
            case["count"].as_u64().unwrap() as usize,
            case["requested"].as_i64(),
        )
        .unwrap();
        if actual.supported {
            assert_ne!(actual.game_mask, 0, "topology: {case}");
            assert_ne!(actual.background_mask, 0, "topology: {case}");
            assert_eq!(
                actual.game_mask | actual.background_mask,
                actual.all_mask,
                "topology: {case}"
            );
        } else {
            assert_eq!(actual.performance_core_count, Some(1), "topology: {case}");
            assert_eq!(actual.efficiency_core_count, Some(0), "topology: {case}");
        }
    }
    for case in cases["keys"].as_array().unwrap() {
        assert_eq!(
            json!(settings::turbo_keys(&case["input"])),
            case["expected"]
        );
    }
}
