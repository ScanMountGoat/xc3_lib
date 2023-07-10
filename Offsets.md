# Offsets (WIP)
## Write Functions
Writing is split into two main functions. The `write` function writes the data and placeholder offset values. This function also calculates an objects size. The `write_offset` function updates the offsets from the previous step and writes the pointed to data. This approach is similar to the two pass measure and layout approach used for user interface layout. The main difference is that addresses in binary files are 1D and the constraints are much simpler.

```python
# TODO: lists fields return a list of offsets instead of just one?
# Automatically generated for each type.
def write(self, writer, data_ptr):
    # The actual implementation uses a type with named fields.
    field_offsets = []

    # Repeat for each field.
    field_offsets.append(writer.position)
    self.field0.write(writer)
    ...

    # Update data_ptr to point past this write
    data_ptr = max(data_ptr, writer.position)
    return field_offsets

# Implemented manually for the handful of generic pointer types.
def write_offset(self, writer, offset, data_ptr):
    # Use data_ptr to update the placeholder offset.
    writer.position = offset
    data_ptr.write(writer)

    # Write the pointed to data and update data_ptr.
    writer.position = data_ptr
    offsets = self.write(writer, data_ptr)
    # TODO: Restore writer position?
    return offsets
```

## VertexData
We always call `write` on each field in the order they are defined.
The calls to `write_offset` must be applied in a specific order to match in game files.

```python
# The writer and data_ptr parameters are omitted from this example.
def write_vertex_data(root):
    root_offsets = root.write()

    # Call write_offset based on the order items appear in the file.
    vertex_buffers_offsets = root.vertex_buffers.write_offset(root_offsets.vertex_buffers)
    root.index_buffers.write_offset(root_offsets.index_buffers)
    root.vertex_buffer_info.write_offset(root_offsets.vertex_buffer_info)
    root.outline_buffers.write_offset(root_offsets.outline_buffers)

    for b, offsets in zip(root.vertex_buffers, vertex_buffers_offsets):
        b.attributes.write_offset(offsets.attributes)

    weights_offsets = root.weights.write_offset(root_offsets.weights)
    for g, offset in zip(root.weights.groups, weights_offsets.groups):
        g.write_offset(offset)

    root.vertex_animation.write_offset()
    root.vertex_animation.descriptors.write_offset()
    root.vertex_animation.targets.write_offset()
    for d in root.vertex_animation.descriptors:
        d.write_offset()

    root.unk.write_offset()
    root.unk.unk1.write_offset()

    root.buffer.write_offset()
```

## Msrd
```python
# The writer and data_ptr parameters are omitted from this example.
def write_msrd(root):
    root_offsets = root.write()

    # Call write_offset based on the order items appear in the file.
    root.stream_entries.write_offset()
    root.streams.write_offset()

    root.texture_resources.write_offset()

    root.texture_ids.write_offset()

    # TODO: Will this always be done in the same way?
    # TODO: Move logic into write_offset of the parent?
    root.textures.write_offset()
    root.textures.textures.write_offset()
    for texture in root.textures.textures:
        texture.name.write_offset()

    for stream in root.streams:
        stream.xbc1.write_offset()
```
