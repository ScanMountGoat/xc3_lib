# xc3_viewer
A simple model and map renderer for Xenoblade 1 DE, Xenoblade 2, and Xenoblade 3.

## Usage
The shader database parameter is optional but highly recommended since the fallback texture assignments do not handle channel assignments.

`xc3_viewer "Xeno 2 Dump\map\ma02a.wismhd" xc2.json`  
`xc3_viewer "Xeno 3 Dump\chr\ch\ch01027000.wimdo" xc3.json`  
`xc3_viewer "Xeno 3 Dump\chr\ch\ch01027000.wimdo" xc3.json --anim "Xeno 3 Dump\chr\ch\ch01027000_event.mot" --anim-index 1`  

Select the GBuffer texture to view using the keys 1-6 and 0 for the shaded view. The current animation can be changed using the page up and page down keys. Restart animation playback using spacebar.