use bevy_egui::egui;
use keystone_blocks::{CodeInspection, CodeLanguage, VplState, inspect_code};

use super::{KeystoneScriptExecutor, RhaiScriptExecutor};
use crate::util::script_types::{ScriptCommand, ScriptState, ScriptStepper};

/// Loads Keystone code into blocks and writes the same blocks out as Rhai.
fn blocks_as_rhai(keystone: &str) -> String {
    let mut state = VplState::default();
    let mut code = keystone.to_owned();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        keystone_blocks::show_vpl_for_code(
            ui,
            &mut state,
            0,
            &mut code,
            None,
            CodeLanguage::Keystone,
        );
    });
    output.textures_delta.clear();
    assert_eq!(code, keystone, "loading must not rewrite the code");
    assert!(
        !state.blocks.is_empty(),
        "{keystone:?} did not load as blocks"
    );

    state.language = CodeLanguage::Rhai;
    keystone_blocks::generate_code_from_state(&state)
}

fn commands(executor: &dyn ScriptStepper, source: &str) -> Vec<String> {
    let mut program = executor
        .compile_step(source, None)
        .unwrap_or_else(|error| panic!("{source}\nfailed to compile: {error:?}"));
    let state = ScriptState::new();
    (0..500)
        .filter_map(|_| program.next(&state))
        .map(|command: ScriptCommand| format!("{command:?}"))
        .collect()
}

const PROGRAMS: &[&str] = &[
    // Variables, a counted loop and an else-if chain.
    "count = 0\nloop 3\n    if count == 0\n        move up\n    else if count == 1\n        move right\n    else\n        dig down\n    end\n    count = count + 1\nend",
    // Operator precedence, `not`, and `while`.
    "n = 1\nwhile n < 20 and not (n == 7)\n    n = n * 2 + 1\n    move left\nend\nsleep 1",
    // Sensors and `place`.
    "if is_empty(down)\n    place down\nelse\n    move right\nend\nif is_touched()\n    move left\nelse\n    sleep 0.5\nend",
    // Nested loops, which need different loop counters in Rhai.
    "loop 2\n    loop 2\n        move up\n    end\n    sleep 0.5\nend\nmove down",
    // `print` sends no command but must still run.
    "word = \"hi\"\nprint word\nmove right",
];

#[test]
fn rhai_written_by_the_blocks_runs_like_the_same_keystone() {
    let keystone = KeystoneScriptExecutor::default();
    let rhai = RhaiScriptExecutor::deterministic();

    for source in PROGRAMS {
        let rhai_source = blocks_as_rhai(source);
        let expected = commands(&keystone, source);
        assert!(!expected.is_empty(), "{source:?} sends no commands");
        assert_eq!(
            commands(&rhai, &rhai_source),
            expected,
            "\n--- keystone\n{source}\n--- rhai\n{rhai_source}"
        );
    }
}

#[test]
fn rhai_written_by_the_blocks_can_be_opened_as_blocks_again() {
    for source in PROGRAMS {
        let rhai_source = blocks_as_rhai(source);
        assert_eq!(
            inspect_code(&rhai_source, CodeLanguage::Rhai),
            CodeInspection::Representable {
                has_comments: false
            },
            "\n{rhai_source}"
        );
    }
}
