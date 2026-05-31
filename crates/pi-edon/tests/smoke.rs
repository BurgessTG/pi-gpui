use std::sync::mpsc;
use std::time::Duration;

use pi_edon::napi::JsString;
use serial_test::serial;

fn node_from_env() -> pi_edon::Result<pi_edon::EmbeddedNode> {
    pi_edon::EmbeddedNode::load_from_env()
}

#[test]
#[serial]
fn evaluates_javascript_when_libnode_is_available() -> Result<(), Box<dyn std::error::Error>> {
    let node = match node_from_env() {
        Ok(node) => node,
        Err(pi_edon::EdonBoundaryError::MissingLibnode) => return Ok(()),
        Err(error) => return Err(Box::new(error)),
    };
    node.eval("globalThis.piGpuiSmoke = 40 + 2;")?;
    node.eval("if (globalThis.piGpuiSmoke !== 42) throw new Error('bad math');")?;
    Ok(())
}

#[test]
#[serial]
fn calls_native_module_from_javascript() -> Result<(), Box<dyn std::error::Error>> {
    let node = match node_from_env() {
        Ok(node) => node,
        Err(pi_edon::EdonBoundaryError::MissingLibnode) => return Ok(()),
        Err(error) => return Err(Box::new(error)),
    };
    let (tx, rx) = mpsc::channel::<String>();
    node.register_module("pi_gpui_smoke", move |env, mut exports| {
        let tx = tx.clone();
        let emit = env.create_function_from_closure("emit", move |ctx| {
            let value = ctx.get::<JsString>(0)?.into_utf8()?.as_str()?.to_owned();
            let _ignored = tx.send(value);
            ctx.env.get_undefined()
        })?;
        exports.set_named_property("emit", emit)?;
        Ok(exports)
    })?;
    node.eval("process._linkedBinding('pi_gpui_smoke').emit('hello-native');")?;
    let value = rx.recv_timeout(Duration::from_secs(5))?;
    assert_eq!(value, "hello-native");
    Ok(())
}
