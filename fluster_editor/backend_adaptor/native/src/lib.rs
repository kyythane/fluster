use neon::prelude::*;

fn initialize(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    Ok(cx.undefined())
}

fn render_frame(mut cx: FunctionContext) -> JsResult<JsArrayBuffer> {
    let frame_index = cx.argument::<JsNumber>(0)?.value();
    let frame_index = if frame_index > std::u32::MAX as f64 {
        return cx.throw_range_error("frameIndex out of bounds");
    } else if frame_index < 0.0 {
        return cx.throw_range_error("frameIndex must be >= 0");
    } else {
        frame_index as u32
    };
    unimplemented!()
}

register_module!(mut cx, {
    cx.export_function("initialize", initialize)?;
    cx.export_function("renderFrame", render_frame)?;
    Ok(())
});
