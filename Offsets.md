# Offsets (WIP)
## Reading Overview
Reading binary files with offsets is generally straightforward. Offsets refer to data elsewhere in the file. Each offset is relative to some starting position like the start of the current struct or the start of the file. The same struct definitions and generated reading code will work across all instances of the file since the positions are encoded in the offsets themselves. In other words, the order of items in the binary file may have little to do with the order of the structs and fields themselves. Some files require multiple passes to parse due to storing types in byte buffers that may also be compressed.

## Writing Overview
Writing binary files with offsets is significantly more challenging. Offsets must be calculated at runtime when writing the file since the lengths and types of data stored in the file may change since the time it was read due to user modifications. Just as offsets can be useful in estimating sizes while reverse engineering files, sizes are important for calculating offsets when writing. See previous work done for the [SSBH binary formats](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_offsets.md) for details.

Offsets in Xenoblade follow certain rules like always being non negative. This means offsets never point backwards. Data items in a file also do not overlap, so each item must ensure the next offset should point past the current data item. Duplicate items are typically handled using an additional layer of indirection such as a list of item indices. These simple rules provide most of the details needing to automate writing data in the next section.

The main challenge with writing is to ensure that writing an unmodified file results in binary identical output to the original. This isn't strictly necessary but makes it substantially simpler to test the writing implementation for errors. 

The standard way to represent binary data in programming languages is in a hierarchy of structs where a root struct has fields that may have offsets to other structs with offsets and so on. This creates a tree structure rooted at the header with nodes or vertices for instances of structs. The offsets between structs define directed edges in the tree. Producing a binary identical output file requires not only calculating offset values that respect the offset rules but also defining an ordering for the data items in the binary file. This ordering can be thought of a tree traversal starting from the root header struct and visiting each data item exactly once. There are many valid strategies to traverse the tree like the traversals used in depth first search (DFS) or breadth first search (BFS). This ordering is currently defined manually due to a lack of any obvious patterns that work across all files.

## Write Functions
Writing is split into two main functions. The `write` function writes the data and placeholder offset values. This function also calculates an objects size. The `write_offset` function updates the offsets from the previous step and writes the pointed to data. This approach is similar to the two pass measure and layout approach used for user interface layout. The main difference is that addresses in binary files are 1D and the constraints are much simpler.

```python
# This function and FieldOffsets can be automatically generated for each type.
def write(self, writer, data_ptr):
    # Store the position of the offset and data for each field.
    field_offsets = FieldOffsets

    # Repeat for each field.
    field_offsets.field0 = Offset(writer.position, self.field0)
    self.field0.write(writer)
    ...

    # Update data_ptr to point past this write
    data_ptr = max(data_ptr, writer.position)
    return field_offsets

def Offset:
    def __init__(self, position, data):
        self.position = position
        self.data = data

    def write_offset(self, writer, data_ptr):
        # Use data_ptr to update the placeholder offset.
        writer.position = self.position
        data_ptr.write(writer)
    
        # Write the pointed to data and update data_ptr.
        writer.position = data_ptr
        offsets = self.data.write(writer, data_ptr)
        return offsets
```

## VertexData
We always call `write` on each field in the order they are defined.
The calls to `write_offset` must be applied in a specific order to match in game files.

```python
# The writer and data_ptr parameters are omitted from this example.
def write_vertex_data(root, ...):
    root_offsets = root.write(...)

    # Call write_offset based on the order items appear in the file.
    vertex_buffers_offsets = root_offsets.vertex_buffers.write_offset(...)
    root_offsets.index_buffers.write_offset(...)
    root_offsets.vertex_buffer_info.write_offset(...)
    root_offsets.outline_buffers.write_offset(...)

    for offsets in vertex_buffers_offsets:
        offsets.attributes.write_offset(...)

    weights_offsets = root_offsets.weights.write_offset(...)
    weights_offsets.groups.write_offset(...)

    vertex_animation_offsets = root_offsets.vertex_animation.write_offset(...)
    descriptors_offsets = vertex_animation_offsets.descriptors.write_offset(...)
    vertex_animation_offsets.targets.write_offset(...)
    for offsets in descriptors_offsets:
        offsets.unk1.write_offset(...)

    unk_offsets = root_offsets.unk.write_offset(...)
    unk_offsets.unk1.write_offset(...)

    root_offsets.buffer.write_offset(...)
```

## Msrd
```python
# The writer and data_ptr parameters are omitted from this example.
def write_msrd(root, ...):
    root_offsets = root.write(...)

    # Call write_offset based on the order items appear in the file.
    root_offsets.stream_entries.write_offset(...)
    root_offsets.streams.write_offset(...)

    root_offsets.texture_resources.write_offset(...)

    root_offsets.texture_ids.write_offset(...)

    # TODO: Will this always be done in the same way?
    # TODO: Move logic into write_offset of the parent?
    root_textures_offsets = root_offsets.textures.write_offset(...)
    textures_offsets = root_textures_offsets.textures.write_offset(...)
    for offsets in textures_offsets:
        offsets.name.write_offset(...)

    for offsets in root_offsets.streams:
        offsets.xbc1.write_offset(...)
```
