# xc3_wgpu
A model and map rendering library using xc3_model and wgpu.

xc3_wgpu is designed to be simple and easy to debug rather than having perfect in game accuracy. The renderer is built to match Xenoblade 3 but works with all supported games due to conversions built into xc3_model to handle differences in materials and shader outputs. Unique shaders are generated for each model at runtime to recreate the assignments and layering for G-Buffer textures. This makes comparing models in debuggers like [RenderDoc](https://renderdoc.org/) between xc3_wgpu and an emulator much easier. Deferred lighting and post processing is currently very basic since the focus is on improving accuracy of model texture assignments and layering that also benefits tools like xc3_gltf or xenoblade_blender.
