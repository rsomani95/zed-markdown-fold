# Heading Level 1
Paragraph under H1.

## Heading Level 2
Paragraph under H2.

### Heading Level 3
Paragraph under H3.

#### Heading Level 4
Paragraph under H4.

##### Heading Level 5
Paragraph under H5.

###### Heading Level 6
Paragraph under H6.

## Sibling After Deep Nesting

This section should cause H2-H6 above to close, but H1 should stay open.

# Heading Hierarchoty

## First Child

Content.

## Second Child

### Grandchild

Grandchild content.

## Third Child (should close Grandchild + Second Child)

Content.

# Code Blocks

## Backtick Fenced

```python
def hello():
    print("world")
```

## Backtick Fenced with No Language

```
plain code
more plain code
```

## Tilde Fenced

~~~
tilde code block
more tilde code
~~~

## Tilde Fenced with Language

~~~rust
fn main() {
    println!("hello");
}
~~~

## Code Block with Headings Inside (should NOT create heading folds)

```markdown
# This is inside a code block
## So is this
### And this
```

## Empty Code Block

```
```

## Single Line Code Block

```
one liner
```

## Indented Code Block (4 spaces, no fence)

    this is an indented code block
    second line
    third line

Regular paragraph after indented block.

# Blockquotes

## Simple Blockquote

> Single line blockquote.

## Simple Blockquote - 2

> Single line blockquote that stretches on to go on to be multiple lines (single line blockquote but displayed over multiple lines cannot be condensed )

## Multi-line Blockquote

> Line one of blockquote
> Line two of blockquote
> Line three of blockquote

## Blockquote with Blank Line Gap

> First paragraph in quote.
>
> Second paragraph in quote (still same blockquote per CommonMark).

## Lazy Continuation Blockquote

> This is a blockquote
that continues without the > prefix
> and then comes back.

## Nested Blockquote

> Outer blockquote
> > Inner blockquote
> > Still inner
> Back to outer

## Blockquote with Heading

> ## Heading inside blockquote
> Content under that heading.

## Blockquote with Code

> ```python
> def foo():
>     pass
> ```

## Blockquote Followed Immediately by Heading

> Some quoted text
> More quoted text

## Adjacent Blockquotes with No Gap

> First blockquote

> Second blockquote (is this separate?)

# Lists

## Unordered List

- Item one
- Item two
- Item three

## Nested Unordered List

- Parent item
  - Child item
  - Another child
    - Grandchild
- Back to top level

## Ordered List

1. First
2. Second
3. Third

## Mixed List

- Unordered
  1. Ordered child
  2. Another ordered
- Back to unordered

## List with Multi-line Items

- This is a list item
  that spans multiple lines
  with continuation.
- Second item also
  has continuation.

## List with Code Block

- Item with code:
  ```python
  x = 1
  ```
- Next item

## List with Blockquote

- Item one
  > Quoted inside list
  > More quoted
- Item two

# Front Matter

---
title: Test Document
date: 2024-01-01
tags: [test, markdown]
---

(This YAML front matter only works at the very top of a file, so it will render as a thematic break + paragraph here. But the extension should handle it at position 0.)

# Tables

## Simple Table

| Column A | Column B | Column C |
|----------|----------|----------|
| a1       | b1       | c1       |
| a2       | b2       | c2       |
| a3       | b3       | c3       |

## Wide Table

| Name | Email | Phone | Address | City | State | Zip | Country |
|------|-------|-------|---------|------|-------|-----|---------|
| Alice | a@b.c | 123 | 1 Main | NYC | NY | 10001 | US |
| Bob | b@b.c | 456 | 2 Main | LA | CA | 90001 | US |

# HTML Blocks

## Inline HTML

<div>
  <p>This is an HTML block.</p>
  <p>It spans multiple lines.</p>
</div>

## Details/Summary (collapsible in GitHub)

<details>
<summary>Click to expand</summary>

This content is inside a details block.
It has multiple lines.

</details>

# Edge Cases

## Empty Section (heading with nothing before next heading)

## Consecutive Headings

### Another Consecutive

#### And Another

Back to content.

## Heading with Inline Formatting

Content here.

## Heading with `inline code` and **bold** and *italic*

Content here.

## Heading with [link](https://example.com)

Content here.

##  Heading with Extra Leading Spaces

Content here.

## Trailing Whitespace on Heading   

Content here.

## Very Long Heading That Goes On And On And On And On And On And On And On And On

Content here.

#NotAHeading (no space after hash)

####### Seven Hashes (not a valid heading)

# Thematic Breaks

Content above.

---

Content between thematic breaks.

***

Content between.

___

Content below.

# Mixed Content Section

Some prose.

> A blockquote in the middle.

More prose.

```
A code block in the middle.
```

More prose.

- A list in the middle
- With items

Final prose.

# Deeply Nested Then Reset

## L2

### L3

#### L4

##### L5

###### L6

Deep content.

# Back to L1 (all above should close)

Final content in the file.
