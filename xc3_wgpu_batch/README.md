# xc3_wgpu_batch
A simple model and map batch renderer for Xenoblade Chronicles X, Xenoblade Chronicles 1 Definitive Edition, Xenoblade Chronicles 2, Xenoblade Chronicles 3, and Xenoblade Chronicles X Definitive Edition.

xc3_wgpu_batch renders `.camdo`, `.pcmdo`, `.wimdo` or `.wismhd` files to PNG in the given folder recursively.
This tests the libraries end-to-end from binary parsing in xc3_lib all the way to final pixels on screen from xc3_wgpu.
Logging is enabled to help with identifying non fatal errors in library code.
The PNG outputs can to be inspected manually to check for rendering errors.

## Usage
The shader database parameter is optional but highly recommended since the fallback texture assignments do not handle channel assignments.

`xc3_wgpu_batch --help`  
`xc3_wgpu_batch "Xeno X Dump" camdo xcx.bin`  
`xc3_wgpu_batch "Xeno 1 Dump" wismhd xc1.bin`  
`xc3_wgpu_batch "Xeno 2 Dump" wimdo xc2.bin`  
`xc3_wgpu_batch "Xeno 3 Dump/chr/ch/ch" wimdo xc3.bin --anim`  

The graphics API can be changed using the `--backend` argument. See `xc3_viewer --help` for details. This mostly applies to Windows users that may have better compatibility using `--backend dx12` instead of the default backend selection.
