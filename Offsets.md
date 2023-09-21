# Offsets (WIP)
This document gives an overview of the implementation used for writing and data layout for binary formats. The goal is to motivate the high level ideas used in the actual Rust implementation. Python style pseudocode is provided to better illustrate the concepts but is not guaranteed to be fully valid or working code. For details, please consult the actual implementation in xc3_lib and xc3_lib derive.

## Reading Overview
Reading binary files with offsets is generally straightforward. Offsets refer to data elsewhere in the file. Each offset is relative to some starting position like the start of the current struct or the start of the file. Matching the order of fields in the struct to the order of fields in the file enables generating the reading code at compile time. The layout of items behind offsets in the file does not matter for reading code since the process of seeking to the data and reading it remains the same. Some files require multiple passes to parse due to storing types in byte buffers that may also be compressed.

## Writing Overview
Writing binary files with offsets is significantly more challenging. Offsets must be calculated at runtime when writing the file since the lengths and types of data stored in the file may change since the time it was read due to user modifications. Unlike reading code, writing code must consider the layout of items behind offsets in the file. Just as offsets can be useful in estimating sizes while reverse engineering files, sizes are important for calculating offsets when writing. See previous work done for the [SSBH binary formats](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_offsets.md) for details.

The main challenge with writing is to ensure that writing an unmodified file results in binary identical output to the original. This isn't strictly necessary but makes it substantially simpler to test the writing implementation for errors. 

Offsets in Xenoblade follow certain rules like always being non negative. This means offsets never point backwards. This does not imply that offsets are strictly increasing when visited in any set order, however. An offset can still point to a position before the offset field itself if the offset is relative to the start of the file.

Data items in a file also do not overlap, so each item must ensure the next offset should point past the current data item. Duplicate items are typically handled using an additional layer of indirection such as a list of item indices. These simple rules provide most of the details needing to automate writing data in the next section.

The standard way to represent binary data in programming languages is in a hierarchy of structs. The root struct has fields that may have offsets to other structs with offsets and so on. This creates a tree structure rooted at the header with nodes or vertices for instances of structs. Whether the "offsets" are actually pointers is an implementation detail. The offsets between structs define directed edges in the tree. 

Producing a binary identical output file requires not only calculating offset values that respect the offset rules but also defining an ordering for the data items in the binary file. This ordering can be thought of a tree traversal starting from the root header struct and visiting each data item exactly once. There are many valid strategies to traverse the tree like the traversals used in depth first search (DFS) or breadth first search (BFS). This ordering is currently defined manually in many cases due different conventions for file layout for different formats.

## Write Functions
Writing is split into two main functions that define two passes. The `write` function writes the data and placeholder offset values. This function also calculates an objects size. Size is assumed to be the difference in the write position before and after writing for simplicity. The return value stores the position of offset values as well as the data they point to for later.

```python
# Mutable writing context.
def Ctx:
    __init__(self):
        self.data_ptr = 0

# This function and FieldOffsets can be automatically generated for each type.
def write(self, writer, ctx: Ctx) -> FieldOffsets:
    # Store the position of the offset and data for each field.
    field_offsets = FieldOffsets()

    # Offset field.
    field_offsets.field0 = Offset(writer.position, self.field0)
    writer.write(0)
    # Regular field.
    self.field1.write(writer, ctx)
    ...

    # Update data_ptr to point past this write.
    # This implicitly calculates an object's size.
    ctx.data_ptr = max(ctx.data_ptr, writer.position)
    return field_offsets

def Offset:
    def __init__(self, position: int, data):
        self.position = position
        self.data = data

    # write_offset_full is similar but calls write_full instead of write.
    def write_offset(self, writer, base_offset: int, ctx: Ctx) -> OffsetsForData:
        # Use data_ptr to update the placeholder offset.
        writer.position = self.position
        offset_value = ctx.data_ptr - base_offset
        writer.write(offset_value)
    
        # Write the pointed to data, potentially updating ctx.data
        writer.position = ctx.data_ptr
        offsets = self.data.write(writer, ctx)
        return offsets
```

The `write_full` function represents a complete write and thus doesn't return any values. Any type that knows how to write itself and its offsets can also implement a complete write with `write_full`. This also applies to types without offset fields and primitive types since they don't need to write any offset data.

```python
def write_full(self, writer, base_offset: int, ctx: Ctx):
    offsets = self.write(writer, data_ptr)
    offsets.write_full(writer, base_offset, ctx.data_ptr)
```

The implementation of `write_full` for the offset return type may need to be implemented manually to match the data layout of existing files. The implementation can be derived if the offset fields are updated in order recursively.

```python
def write_full(self, writer, base_offset: int, ctx: Ctx):
    # It may be necessary to defer writing inner offsets until later.
    field0_offsets = self.field0.write_offset(writer, base_offset, ctx)
    self.field1.write_offset_full(writer, base_offset, ctx)
    field0_offsets.name.write_offset_full(writer, base_offset, ctx)
    ...
```
