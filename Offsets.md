# Offsets (WIP)
This document gives an overview of the implementation used for writing and data layout for binary formats. The goal is to motivate the high level ideas used in the actual Rust implementation. Python style pseudocode is provided to better illustrate the concepts but is not guaranteed to be fully valid or working code. For details, please consult the actual implementation in xc3_lib and xc3_lib derive.

## Reading Overview
Reading binary files with offsets is generally straightforward. Offsets refer to data elsewhere in the file. Each offset is relative to some starting position like the start of the current struct or the start of the file. Matching the order of fields in the struct to the order of fields in the file enables generating the reading code at compile time. The layout of items behind offsets in the file does not matter for reading code since the process of seeking to the data and reading it remains the same. Some files require multiple passes to parse due to storing types in byte buffers that may also be compressed.

## Writing Overview
Writing binary files with offsets is significantly more challenging. Offsets must be calculated at runtime when writing the file since the lengths and types of data stored in the file may change since the time it was read due to user modifications. Unlike reading code, writing code must consider the layout of items behind offsets in the file. Just as offsets can be useful in estimating sizes while reverse engineering files, sizes are important for calculating offsets when writing. See previous work done for the [SSBH binary formats](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_offsets.md) for details.

## Offset Rules
Offsets in Xenoblade follow certain rules like always being non negative. This means offsets never point backwards. This does not imply that offsets are strictly increasing when visited in any set order, however. An offset can still point to a position before the offset field itself if the offset is relative to the start of the file. Data items in a file also do not overlap, so each item must ensure the next offset should point past the current data item. Duplicate items are typically handled using an additional layer of indirection such as a list of item indices. These simple rules provide most of the details needing to automate writing data in the next section.

## Data Layout and Ordering
The main challenge with writing is to ensure that writing an unmodified file results in binary identical output to the original. There are many valid ways to calculate offsets and position data in a file. A valid output is one that produces the original data when read again. This can be tested by diffing the debug representation of both files after reading. Exactly matching the original file's bytes isn't strictly necessary but makes it substantially simpler to test the writing implementation for errors. 

Producing a binary identical output file requires not only calculating a valid layout but also the correct ordering for the data items. The reading code visits each field in order recursively. The writing code keeps incrementing the location for the next write and must visit data items in increasing order by absolute offset. This ordering is currently defined manually in many cases due to different layout conventions for different formats. For example, matching the ordering of items in a file may require writing fields in reverse order or writing an inner name offset after all other items have been written.

## Write Functions
Writing is split into two main functions that define two passes. The `write` function defines the first pass that writes the data and placeholder offset values. This function also calculates an objects size. Size is assumed to be the difference in the write position before and after writing for simplicity. The return value stores the position of offset values as well as the data they point to for later.

```python
# Mutable writing context.
def Ctx:
    __init__(self):
        self.data_ptr = 0

def Offset:
    def __init__(self, position: int, data):
        self.position = position
        self.data = data

    # Write one level of offsets.
    def write_offset(self, writer, base_offset: int, ctx: Ctx) -> OffsetsForData:
        # Use data_ptr to update the placeholder offset.
        writer.position = self.position
        offset_value = ctx.data_ptr - base_offset
        writer.write(offset_value)
    
        # Write the pointed to data, potentially updating ctx.data
        writer.position = ctx.data_ptr
        offsets = self.data.write(writer, ctx)
        return offsets

    # Write all levels recursively.
    def write_full(self, writer, base_offset: int, ctx: Ctx) -> OffsetsForData:
        # Use data_ptr to update the placeholder offset.
        writer.position = self.position
        offset_value = ctx.data_ptr - base_offset
        writer.write(offset_value)
    
        # Write the pointed to data, potentially updating ctx.data
        writer.position = ctx.data_ptr
        write_full(self.data, writer, ctx)
        return offsets

# This function and FieldOffsets can be automatically generated for each type.
def Root:
    def write(self, writer, ctx: Ctx) -> RootOffsets:
        # Store the position of the offset and data for each field.
        offsets = RootOffsets()

        # Offset field.
        offsets.field0 = Offset(writer.position, self.field0)
        writer.write(0)
        # Regular field.
        self.field1.write(writer, ctx)
        ...

        # Update data_ptr to point past this write.
        # This implicitly calculates an object's size.
        ctx.data_ptr = max(ctx.data_ptr, writer.position)
        return offsets
```

The `write_offsets` function defines the second pass that updates the placeholder offsets and writes the pointed to data recursively. This function should write each offset field in increasing order by absolute offset. The implementation of `write_offsets` for the offset return type may need to be implemented manually to match the data layout of existing files. The implementation can be derived if the offset fields are updated in order recursively. 

```python
def RootOffsets:
    def write_offsets(self, writer, base_offset: int, ctx: Ctx):
        # It may be necessary to defer writing inner offsets until later.
        field0_offsets = self.field0.write_offset(writer, base_offset, ctx)

        # Types without special ordering use the recursively defined write_full.
        self.field1.write_full(writer, base_offset, ctx)

        field0_offsets.name.write_full(writer, base_offset, ctx)
    ...
```

The `write_full` function represents a complete write and thus doesn't return any values. Any type that knows how to write itself and its offsets can also implement a complete write with `write_full`. This also applies to types without offset fields and primitive types since they don't need to write any offset data.

```python
def write_full(self, writer, base_offset: int, ctx: Ctx):
    offsets = self.write(writer, ctx)
    offsets.write_offsets(writer, base_offset, ctx.data_ptr)
```

This two pass approach is similar to the measure and layout passes used in some UI frameworks. Having separate passes for offsets adds more flexibility in data layout and ordering. A single pass approach simplifies the implementation but requires making additional layout and ordering assumptions. See [SsbhWrite](https://github.com/ultimate-research/ssbh_lib/blob/master/ssbh_offsets.md) for an example of a single pass implementation.