//! `verification` refusals carry a typed `error.kind` through served dispatch
//! (#741), and leave board/project files untouched. Board fixtures are KiCad 10
//! resaves (`specctra_two_resistors.kicad_pcb`, `specctra_two_resistors_locked.kicad_pcb`).

use super::{handle_check_clearance, handle_set_predefined_sizes, open_board, Looking};
use crate::mcp::handler::McpHandler;
use crate::tools::project_file_writer_tests::{body_of, served_call};
use crate::tools::{ServerConfig, ToolContext, ToolRouter};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LOCKED_BOARD: &str = "specctra_two_resistors_locked.kicad_pcb";
const LOCKED_VIA_UUID: &str = "db24f42e-63bc-4e53-a1a7-509928ef3cd6";
const TWO_RESISTORS_BOARD: &str = "specctra_two_resistors.kicad_pcb";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn copy_fixture(name: &str) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let source = fixture(name);
    let path = dir.path().join(name);
    std::fs::write(&path, &source).unwrap();
    (dir, path, source)
}

fn blank_project() -> &'static str {
    r#"{
  "board": {
    "design_settings": {
      "track_widths": [0.0],
      "via_dimensions": [{ "diameter": 0.0, "drill": 0.0 }]
    }
  },
  "meta": { "filename": "specctra_two_resistors.kicad_pro", "version": 3 }
}"#
}

async fn handler_with(config: ServerConfig) -> McpHandler {
    McpHandler::new(ServerConfig {
        eager_toolsets: true,
        ..config
    })
    .await
    .unwrap()
}

async fn handler() -> McpHandler {
    handler_with(ServerConfig::default()).await
}

fn direct_ctx() -> ToolContext {
    ToolContext::new(ServerConfig::default(), Arc::new(ToolRouter::new()))
}

#[tokio::test]
async fn predefined_sizes_without_project_file_returns_file_not_found() {
    let (_dir, board, original_board) = copy_fixture(TWO_RESISTORS_BOARD);
    let expected_project = board.with_extension("kicad_pro");
    let expected_path = copperpour_sexp::paths::display_path(&expected_project);
    let h = handler().await;

    for (tool, args) in [
        (
            "set_predefined_sizes",
            json!({ "board": board, "track_widths": [0.25] }),
        ),
        ("get_predefined_sizes", json!({ "board": board })),
    ] {
        let result = served_call(&h, tool, args).await;
        assert!(result.is_error, "{tool} should refuse");
        let body = body_of(&result);
        assert_eq!(body["error"]["kind"], "file_not_found", "{tool}: {body}");
        assert_eq!(body["error"]["path"], expected_path, "{tool}: {body}");
        assert_eq!(
            body["error"]["absolute_path"], expected_path,
            "{tool}: {body}"
        );
        assert!(
            body["message"].as_str().unwrap().starts_with(&format!(
                "No project file at {}",
                expected_project.display()
            )),
            "{tool}: {body}"
        );
        assert_eq!(std::fs::read_to_string(&board).unwrap(), original_board);
        assert!(!expected_project.exists());
    }
}

#[tokio::test]
async fn set_predefined_sizes_refuses_invalid_values_through_served_dispatch() {
    let (_dir, board, _) = copy_fixture(TWO_RESISTORS_BOARD);
    let project = board.with_extension("kicad_pro");
    std::fs::write(&project, blank_project()).unwrap();
    let h = handler().await;

    let cases: &[(&str, Value, &str, &str)] = &[
        (
            "neither list given",
            json!({ "board": board }),
            "track_widths",
            "Argument 'track_widths' is invalid: name at least one of track_widths, via_dimensions",
        ),
        (
            "negative track width after valid entry",
            json!({ "board": board, "track_widths": [0.2, -0.5] }),
            "track_widths[1]",
            "Argument 'track_widths[1]' is invalid: must be a finite number greater than 0 mm, got -0.5",
        ),
        (
            "negative via diameter",
            json!({ "board": board, "via_dimensions": [{ "diameter": -0.6, "drill": 0.3 }] }),
            "via_dimensions[0].diameter",
            "Argument 'via_dimensions[0].diameter' is invalid: must be a finite number greater than 0 mm, got -0.6",
        ),
        (
            "negative via drill",
            json!({ "board": board, "via_dimensions": [{ "diameter": 0.6, "drill": -0.3 }] }),
            "via_dimensions[0].drill",
            "Argument 'via_dimensions[0].drill' is invalid: must be a finite number greater than 0 mm, got -0.3",
        ),
        (
            "diameter not greater than drill on second via",
            json!({
                "board": board,
                "via_dimensions": [
                    { "diameter": 0.6, "drill": 0.3 },
                    { "diameter": 0.3, "drill": 0.3 }
                ]
            }),
            "via_dimensions[1]",
            "Argument 'via_dimensions[1]' is invalid: diameter (0.3 mm) must be greater than drill (0.3 mm)",
        ),
    ];

    for (label, args, expected_field, expected_message) in cases {
        let result = served_call(&h, "set_predefined_sizes", args.clone()).await;
        assert!(result.is_error, "{label} should refuse");
        let body = body_of(&result);
        assert_eq!(body["error"]["kind"], "invalid_argument", "{label}: {body}");
        assert_eq!(body["error"]["field"], *expected_field, "{label}: {body}");
        assert_eq!(body["message"], *expected_message, "{label}: {body}");
        assert_eq!(std::fs::read_to_string(&project).unwrap(), blank_project());
    }
}

/// Direct-call coverage for type checks that served schema validation can mask.
#[tokio::test]
async fn set_predefined_sizes_refuses_wrong_types_in_direct_calls() {
    let (_dir, board, _) = copy_fixture(TWO_RESISTORS_BOARD);
    let project = board.with_extension("kicad_pro");
    std::fs::write(&project, blank_project()).unwrap();
    let ctx = direct_ctx();

    let cases: &[(&str, Value, &str, &str)] = &[
        (
            "track_widths not an array",
            json!({ "board": board, "track_widths": 0.2 }),
            "track_widths",
            "Argument 'track_widths' is invalid: missing or not an array",
        ),
        (
            "track_widths entry not a number",
            json!({ "board": board, "track_widths": [0.2, "wide"] }),
            "track_widths[1]",
            "Argument 'track_widths[1]' is invalid: missing or not a number",
        ),
        (
            "via_dimensions not an array",
            json!({ "board": board, "via_dimensions": "0.6/0.3" }),
            "via_dimensions",
            "Argument 'via_dimensions' is invalid: missing or not an array",
        ),
        (
            "via_dimensions entry not an object",
            json!({ "board": board, "via_dimensions": [{ "diameter": 0.6, "drill": 0.3 }, 42] }),
            "via_dimensions[1]",
            "Argument 'via_dimensions[1]' is invalid: missing or not an object",
        ),
        (
            "via_dimensions diameter missing",
            json!({ "board": board, "via_dimensions": [{ "drill": 0.3 }] }),
            "via_dimensions[0].diameter",
            "Argument 'via_dimensions[0].diameter' is invalid: missing or not a number",
        ),
        (
            "via_dimensions drill missing",
            json!({ "board": board, "via_dimensions": [{ "diameter": 0.6 }] }),
            "via_dimensions[0].drill",
            "Argument 'via_dimensions[0].drill' is invalid: missing or not a number",
        ),
    ];

    for (label, args, expected_field, expected_message) in cases {
        let result = handle_set_predefined_sizes(args, &ctx).await.unwrap();
        assert!(result.is_error, "{label} should refuse");
        let body = body_of(&result);
        assert_eq!(body["error"]["kind"], "invalid_argument", "{label}: {body}");
        assert_eq!(body["error"]["field"], *expected_field, "{label}: {body}");
        assert_eq!(body["message"], *expected_message, "{label}: {body}");
        assert_eq!(std::fs::read_to_string(&project).unwrap(), blank_project());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn launch_kicad_ui_spawn_failure_returns_editor_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    // Must exist as a regular file so `find_gui` resolves it instead of falling
    // back to a system KiCad install; non-executable mode triggers spawn error.
    let unexecutable = dir.path().join("kicad");
    std::fs::write(&unexecutable, "").unwrap();
    std::fs::set_permissions(&unexecutable, std::fs::Permissions::from_mode(0o644)).unwrap();

    let h = handler_with(ServerConfig {
        kicad_binary: unexecutable.display().to_string(),
        ..ServerConfig::default()
    })
    .await;

    let result = served_call(&h, "launch_kicad_ui", json!({ "wait_ready": false })).await;
    assert!(result.is_error, "unexecutable binary should refuse");
    let body = body_of(&result);
    assert_eq!(body["error"]["kind"], "editor_unavailable", "{body}");
    assert_eq!(body["error"]["editor"], "kicad", "{body}");
    let message = body["message"].as_str().unwrap();
    assert_eq!(body["error"]["reason"], message, "{body}");
    assert!(
        message.starts_with(&format!(
            "Failed to launch KiCAD ({}): ",
            unexecutable.display()
        )),
        "{body}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn open_board_spawn_failure_returns_editor_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, board, _) = copy_fixture(TWO_RESISTORS_BOARD);
    let bin_dir = tempfile::tempdir().unwrap();
    let kicad = bin_dir.path().join("kicad");
    let pcbnew = bin_dir.path().join("pcbnew");
    std::fs::write(&kicad, "").unwrap();
    std::fs::write(&pcbnew, "").unwrap();
    std::fs::set_permissions(&pcbnew, std::fs::Permissions::from_mode(0o644)).unwrap();

    let ctx = ToolContext::new(
        ServerConfig {
            kicad_binary: kicad.display().to_string(),
            ipc_address: String::new(),
            ..ServerConfig::default()
        },
        Arc::new(ToolRouter::new()),
    );
    ctx.enable_ipc_rediscovery();

    let result = open_board(
        &json!({ "board": board, "timeout_seconds": 1 }),
        &ctx,
        Looking {
            endpoints: Box::new(|| {
                vec!["inproc://copperpour-verification-open-board-spawn-fail".to_string()]
            }),
            unreachable_editor: None,
        },
    )
    .await
    .unwrap();
    assert!(result.is_error, "unexecutable pcb editor should refuse");
    let body = body_of(&result);
    assert_eq!(body["error"]["kind"], "editor_unavailable", "{body}");
    assert_eq!(body["error"]["editor"], "pcb", "{body}");
    let message = body["message"].as_str().unwrap();
    assert_eq!(body["error"]["reason"], message, "{body}");
    assert!(
        message.starts_with(&format!(
            "Failed to launch KiCad's PCB editor ({}): ",
            pcbnew.display()
        )),
        "{body}"
    );
}

#[tokio::test]
async fn copy_routing_pattern_malformed_board_returns_malformed_file() {
    let (_dir, board, original) = copy_fixture(LOCKED_BOARD);
    let h = handler().await;

    // 1. Unbalanced root form.
    let truncated = format!("{original}(");
    std::fs::write(&board, &truncated).unwrap();
    let result = served_call(
        &h,
        "copy_routing_pattern",
        json!({
            "board": board,
            "src_x1": 99.0, "src_y1": 48.0,
            "src_x2": 106.0, "src_y2": 51.0,
            "dest_x": 101.5, "dest_y": 53.0,
        }),
    )
    .await;
    assert!(result.is_error, "truncated board should refuse");
    let body = body_of(&result);
    assert_eq!(body["error"]["kind"], "malformed_file", "{body}");
    assert_eq!(body["error"]["path"], board.display().to_string(), "{body}");
    let reason = body["error"]["reason"].as_str().unwrap();
    assert!(!reason.is_empty(), "{body}");
    assert_eq!(
        body["message"],
        format!("Refusing to copy routing: {reason}. The board file was not modified."),
        "{body}"
    );
    assert_eq!(std::fs::read_to_string(&board).unwrap(), truncated);

    // 2. Routing item without uuid.
    let no_uuid = original.replacen(&format!("(uuid \"{LOCKED_VIA_UUID}\")"), "", 1);
    assert_ne!(no_uuid, original, "fixture premise changed");
    std::fs::write(&board, &no_uuid).unwrap();
    let result = served_call(
        &h,
        "copy_routing_pattern",
        json!({
            "board": board,
            "src_x1": 99.0, "src_y1": 48.0,
            "src_x2": 106.0, "src_y2": 51.0,
            "dest_x": 101.5, "dest_y": 53.0,
        }),
    )
    .await;
    assert!(result.is_error, "via without uuid should refuse");
    let body = body_of(&result);
    assert_eq!(body["error"]["kind"], "malformed_file", "{body}");
    assert_eq!(body["error"]["path"], board.display().to_string(), "{body}");
    assert_eq!(body["error"]["reason"], "a via has no uuid", "{body}");
    assert_eq!(
        body["message"],
        "Refusing to copy routing: a via has no uuid. The board file was not modified.",
        "{body}"
    );
    assert_eq!(std::fs::read_to_string(&board).unwrap(), no_uuid);
}

#[tokio::test]
async fn check_clearance_invalid_mode_returns_invalid_argument() {
    let (_dir, board, _) = copy_fixture(TWO_RESISTORS_BOARD);
    let h = handler().await;

    // Through served dispatch.
    let served = served_call(
        &h,
        "check_clearance",
        json!({
            "board": board,
            "ref1": "R1",
            "ref2": "R2",
            "mode": "copper"
        }),
    )
    .await;
    assert!(served.is_error, "invalid mode should refuse");
    let served_body = body_of(&served);
    assert_eq!(
        served_body["error"]["kind"], "invalid_argument",
        "{served_body}"
    );
    assert_eq!(served_body["error"]["field"], "mode", "{served_body}");

    // Also direct call, in case served schema validation intercepts enum values.
    let direct = handle_check_clearance(
        &json!({
            "board": board,
            "ref1": "R1",
            "ref2": "R2",
            "mode": "copper"
        }),
        &direct_ctx(),
    )
    .await
    .unwrap();
    assert!(direct.is_error, "direct invalid mode should refuse");
    let direct_body = body_of(&direct);
    assert_eq!(
        direct_body["error"]["kind"], "invalid_argument",
        "{direct_body}"
    );
    assert_eq!(direct_body["error"]["field"], "mode", "{direct_body}");
    assert_eq!(
        direct_body["error"]["reason"], "expected 'anchor' or 'courtyard', got 'copper'",
        "{direct_body}"
    );
    assert_eq!(
        direct_body["message"], "Invalid mode 'copper'; expected 'anchor' or 'courtyard'",
        "{direct_body}"
    );
}
