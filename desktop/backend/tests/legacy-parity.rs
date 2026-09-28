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
        let actual = topology::build(
            &cpus,
            case["count"].as_u64().unwrap() as usize,
            case["requested"].as_i64(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            case["expected"],
            "topology: {case}"
        );
    }
    for case in cases["keys"].as_array().unwrap() {
        assert_eq!(
            json!(settings::turbo_keys(&case["input"])),
            case["expected"]
        );
    }
}
