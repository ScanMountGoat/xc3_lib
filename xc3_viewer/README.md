# xc3_viewer
A simple model and map renderer for Xenoblade X, Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.

## Usage
The shader database parameter is optional but highly recommended since the fallback texture assignments do not handle channel assignments.

`xc3_viewer "Xeno 2 Dump\map\ma02a.wismhd" --database xc2.json`  
`xc3_viewer "Xeno 3 Dump\chr\ch\ch01027000.wimdo" --database xc3.json`  
`xc3_viewer "Xeno 3 Dump\chr\ch\ch01027000.wimdo" --database xc3.json --anim "Xeno 3 Dump\chr\ch\ch01027000_event.mot" --anim-index 1`  

Some `.wimdo` or `.camdo` models are split into multiple files that need to be loaded together.  
`xc3_viewer pc010109.wimdo pc010201.wimdo pc010202.wimdo pc010203.wimdo pc010204.wimdo pc010205.wimdo --database xc1.json`  

Select the [G-Buffer texture](https://scanmountgoat.github.io/xenoblade-rendering-research/gbuffer) to view using the keys 1-6 and 0 for the shaded view. Isolate a color channel for modes other than 0 using the RGBA or XYZW keys. Changing the selected texture also resets the color channels. The current animation can be changed using the `,` and `.` keys. Restart animation playback using spacebar.

| Key | Name | Channels |
| --- | --- | --- |
| 0 | Shaded | *lighting and shading with no channel selection* |
| 1 | Color | RGB = color, A = ??? |
| 2 | Etc Params | R = metalness, G = glossiness, B = material ID, A = bit info |
| 3 | Normal | RG = normal, B = ambient occlusion, A = ??? |
| 4 | Velocity | RGB = velocity, A = ??? |
| 5 | Depth | RG = depth, B = ???, A = ??? |
| 6 | Emission | RGB = color, A = ??? |
| 7 | Specular | RGB = color, A = ??? |
