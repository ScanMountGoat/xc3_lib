# xc3_wgpu_batch
A simple model and map batch renderer for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.

xc3_wgpu_batch renders `.wimdo` or `.wismhd` files to PNG in the given folder recursively.
This tests the libraries end-to-end from binary parsing in xc3_lib all the way to final pixels on screen from xc3_wgpu.
Logging is enabled to help with identifying non fatal errors in library code.
The PNG outputs can to be inspected manually to check for rendering errors.

## Usage
The shader database parameter is optional but highly recommended since the fallback texture assignments do not handle channel assignments.

`xc3_wgpu_batch --help`  
`xc3_wgpu_batch "Xeno X Dump" camdo xcx.json`  
`xc3_wgpu_batch "Xeno 1 Dump" wismhd xc1.json`  
`xc3_wgpu_batch "Xeno 2 Dump" wimdo xc2.json`  
`xc3_wgpu_batch "Xeno 3 Dump\chr\ch\ch" wimdo xc3.json --anim`  
