# Introduction

Org mode structural parser/emitter with an emphasis on modularity and avoiding
edits unrelated to changes.

The goal of this library is to parse very efficiently, and edit acceptably
efficiently, to support workflows that parse a large file, make a few changes,
then save, without spurious deltas in the file.

# Features

* Fast minimal structural parser that splits file into a tree of headlines.

* Every UTF-8 string is a valid input, and `emit(parse(text)) == text` for all
  UTF-8 strings.

* Unmodified headlines will be emitted exactly as they were input, even if other
  headlines were changed. (Note that there are edge cases related to which
  section a newline is part of).

* With `headline-parser` flag adds a parser/generator for headlines (tags,
  keyword, priority, planning, etc) that functions on top the structural tree.

* With `orgize-integration` flag, uses
  [orgize](https://github.com/PoiScript/orgize) to parse/generate properties
  drawer.

* Headlines are represented in memory as text, making both parsing and emitting
  very fast, and permitting a two-way mapping between text offset and each
  headline that remains valid even as the in-memory document is modified.

* Arena allocator provides fast performance and precise control over memory.

* Copy-on-write text storage using [Ropey](https://github.com/cessen/ropey).

* Reparse based edit model ensures that the few tree invariants we have are
  never broken, that the in-memory format cannot represent invalid state, and
  that edits which could change the tree structure unintentionally are not
  allowed. For example, changing the text of a node such that it parses into
  multiple nodes is rejected, but any other change is allowed. Should such a
  change be desirable, functions are provided that manipulate the tree
  structure directly.

  This helps limit the blast radius of bugs to the headline(s) affected, even if
  the bug itself results in adding new text that could be parsed as a headline.

# Limitations

* Parses only a small subset of Org mode.

    I have no plans to extend this, except possibly adding native parsing of
    properties rather than relying on Orgize. I recommend using
    [orgize](https://github.com/PoiScript/orgize) to parse section contents.

* Since sections are stored as text, every change to a headline requires
  reparsing the entire headline. A builder is provided to batch such changes if
  desired.

* The fuzz test produces many uninteresting cases where Orgize and Starsector
  parse the same differently. There is some logic to filter out known
  differences, but the fuzz test remains fairly noisy, and requires going
  through it manually to determine whether a difference is actually new. It's
  still very useful, but it's not expected that it will produce no violations.

# Getting Started

See `examples/edit.rs` for a comprehensive example on parsing and editing.

# Arena

Text is stored using a rope, which allows sharing with other Arenas as well as
other code. This also allows multiple versions of a document to be stored
efficiently. `Section::clone_subtree` is helpful here.

The API is currently built around
[IndexTree](https://docs.rs/indextree/latest/indextree/) to model the tree
structure. This means that nodes refer to other nodes by identifier, rather than
by content, and that you can change the text of a document by changing only a
node within it. Multiple documents may be stored in a given Arena, and it can be
thought of as a sort of "builder" for trees.

As such, the only mutable state is stored in the `Arena`. Specific nodes are
referred to with the type `Section` (if you're familiar with `IndexTree`, this
is a wrapper around `NodeId`), which consists of an identifier into the tree.
Most functions are called on `Section`, and take its `Arena` by reference (or
mutable reference).

Although we may reuse `IndexTree` nodes internally, any `Section` provided to
client code is guaranteed to remain valid as long as the Arena lives. This means
that, e.g., you can remove a `Section` from a document, but it will remain
valid, so you could later attach it to another document, or elsewhere in the
same one.

This means that over time, the Arena will accumulate nodes. They are quite small
so this is unlikely to be a problem, but it may be necessary with long lived
Arenas that undergo many edits (inotify-based reparsing, etc) to periodically
emit text, create a new arena, and reparse. If this is an inconvenience, we
could look at adding a convenience function for this -- the main tricky part is
that `Sections` all need to be re-numbered, since preserving existing ones would
require copying all data, defeating the purpose.

# Layered Parsing

There is no one specification for the Org format. The
[draft spec](https://orgmode.org/worg/dev/org-syntax.html), `org-element.el`, and Org
mode commands disagree on how to handle certain edge cases. Different Org
mode commnads may even be inconsistent among themselves. Yet in practice, the
behavior is usually consistent, and in the cases where it varies, it is unlikely
that the user would notice or care. See the [Orgize issue tracker](https://github.com/PoiScript/orgize/issues) (open and
closed) for examples.

Rather than attempt to produce a single parse tree that agrees on all edge
cases, this project takes a layered approach consisting of a structure parser
for the entire file, a headline parser, and properties parser that currently
uses Orgize. Orgize can also be used to fully parse the contents of a headline.

Org mode itself does not operate on a parse tree. Commands are written to
operate on raw text, which makes it possible for different commands to interpet
the grammar differently. While frustrating to a parser, this approach does
provide strong abstraction. Org mode grew out of a text editor, and in many ways
its commands can be thought of as highly specialized editing commands that,
being invoked by a user, can take context into account. It's not how I'd design
it, but I have to admit there is an elegance to it.

Hence, we take a similar approach: Parse the structure into a tree of chunks of
text, and let client code decide what to do with it.

## Structural Parser

The structural parser uses the bare minimum grammar necessary to split the file
into a tree of headlines. We refer to each headline as a *section* consisting of
the line with the stars itself and all text below that line until the next
headline (or end of file). Sections are organized into a tree structure, with
child headlines represented as children of their parent section. There is also a
special section at the root of the document that does not correspond to a
headline, with level 0. Level refers to the number of stars in the headline.

The semantics were chosen to match Org mode as closely as possible. In
particular, a newline refers only to `'\n'`, and literal ASCII space `' '` must
follow the stars. Despite this, Unicode should be fully supported, albeit with a
specific interpretation of significant whitespace.

The subtree rooted at any section can be emitted as an Org file by calling
`to_rope`. Since sections are stored as text, this just traverses the tree in
order and concatenates each section with a newline. This will produce identical
text to the input except for three newline edge cases.

The document itself can also be emitted as an Org file, but it handles those
three edge cases by storing additional state from the original parse, such that
`emit(parse(text)) == text` for all UTF-8 input. If the document is modified, it
will have the same three edge cases.

This design allows us to model all edge cases in the document, meaning that
headlines can be freely added, moved, deleted, and edited while maintaining the
"just concatenate the chunks of text" invariant.

Sections are stored as plain text. The text may be modified directly with
`set_level` and `set_raw`, provided such modification does not break the tree
invarant. For example, removing star from a headline would only be allowed if
the new section still has strictly more stars than the level above it. Likewise,
changing the text to become multiple sections is not permitted by editing the
raw text, structural editing commands (`append`, `prepend`, etc) that operate on
the tree structure must be used instead.

This restriction is a feature, since it means that client code which operates on
a section cannot cause changes in any other section, nor can they corrupt the
tree structure if a bug accidentally introduces a line starting with star into
the body. This makes it easier to write programs which safely read and write
large or complex org mode files frequently, by isolating their changes.

## Headline Parser

In most cases, operating on the raw text will be inconvenient. Often we wish to
operate only on the text under the headline, or only on the headline itself to
change priority, keyword, tags, etc.

When the `headline-parser` feature flag is enabled (default), headline editing
commands become available. These commands are built on top of the structural
parser, and parse a single headline at a time. Each time an accessor is called,
we parse the headline and return it. To modify a headline, we parse the
headline, change it, emit the new headline as text, and then replace the text of
the section with the headline.

We choose this approach so that the headline parser does not need to satisfy the
identity invariant we provide for the overall file. Additionally, headline
parsing brings many edge cases that vary between implementations, and even if
they were consistent, it would be labor intensive to handle correctly. This
design means that headlines which client code change will be interpreted and
reformatted in a standardized way, but only modified headlines will be affected.

As a convenience, individual changes may be made by calling `set_keyword`,
`set_priority`, etc on the section. It is also possible to get a `Headline` by
calling `parse_headline`. This is a value type which provides access to the
headline properties (including the body text). Calling `to_builder` provides a
`HeadlineBuilder` which may be used to change multiple properties at once,
before building a new headline by calling `headline` on it. You can then call
`set_headline` on the Section.

As with changing the section's raw text, edits which break tree invariants will
fail.

## Properties Parser

With `orgize-integration` feature flag (enabled by default), functions that get
and properties (in the properties drawer) become available on both `Section` and
`HeadlineBuilder`. These work similarly to the headline parser, except that they
rely on [Orgize](https://github.com/PoiScript/orgize) to parse the headline.

I'd like to implement my own parser for these as I've done for headlines and
planning, since this has the potential to reformat the entire headline.

# Future Plans

I would like to implement my own parsing for properties drawers to integrate
them better. I have no plans to replicate any other Orgize functionality.

A copy-on-write API for editing trees would be interesting, but I've been
unhappy with previous prototypes along those lines. [Ropey](https://github.com/cessen/ropey) seems to make it work
well, however, so it can be done.

Test coverage is quite solid for the structural parser, and adequate for the
headline parser, but the APIs built on top of them could use more coverage
(possibly doubling as documentation/examples).
