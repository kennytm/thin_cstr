Pre-RFC: Make `*CStr` a Thin Pointer
====================================

## Summary
[summary]: #summary

Make `*CStr` a thin pointer via extern type ([RFC 1861]). `CStr::from_ptr()` will become zero-cost,
while `CStr::to_bytes()` will incur a length calculation.

[RFC 1861]: https://github.com/rust-lang/rfcs/blob/master/text/1861-extern-types.md

## Motivation
[motivation]: #motivation

The `CStr` type was introduced in [RFC 592] during Rust 1.0-alpha as a replacement of the slice type
`[c_char]`, where one of the motivations was

> … in order to construct a slice (or a dynamically sized newtype wrapping a slice), its length has
> to be determined, which is unnecessary for the consuming FFI function that will only receive a
> thin pointer. …

However, Rust at that time only supported three kinds of dynamic-sized types: `str`, `[T]` and trait
objects, where all of them become fat pointers when referenced. An attempt to introduce DST with
thin pointer was made as [RFC 709], but due to time constraint close to the release of 1.0, it was
postponed and kept as a low-priority issue.

Thus the implementation of `CStr` chose to wrap a `[c_char]` and provides the following [FIXME]:

```rust
pub struct CStr {
    // FIXME: this should not be represented with a DST slice but rather with
    //        just a raw `c_char` along with some form of marker to make
    //        this an unsized type. Essentially `sizeof(&CStr)` should be the
    //        same as `sizeof(&c_char)` but `CStr` should be an unsized type.
    inner: [c_char]
}
```

Fast forward to 2017, `extern type` ([RFC 1861]) was introduced to represent opaque FFI types which
are fairly popular in C as a way to hide implementation detail. These types have unspecified size in
the public interface, and also are represented as thin pointers. The `extern type` RFC was accepted
and implemented as an unstable feature in Rust 1.23.

With the introduction of `extern type`, suddenly we have a way to fix the FIXME by changing the
inner slice into such extern type:

```rust
extern {
    type CStrInner;
}
#[repr(C)]
pub struct CStr {
    inner: CStrInner,
}
```

Thus this RFC is proposed to gauge interest if we really want to fix this issue, and sort out
potential unsafety before merging into the standard library.

[RFC 592]: https://github.com/rust-lang/rfcs/blob/master/text/0592-c-str-deref.md
[RFC 709]: https://github.com/rust-lang/rfcs/pull/709
[FIXME]: https://github.com/rust-lang/rust/blob/1410d5604042b739f02f9ec0f2a6c5125c797d52/src/libstd/ffi/c_str.rs#L203-L209

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The main implication of making `*CStr` thin is that the length is no longer stored alongside the
pointer. Some signficant changes are:

* `CStr` becomes `#[repr(C)]` and its pointer type should be compatible with `char*` in C.
* `CStr::from_ptr` becomes free.
* `CStr::to_bytes` and other getter methods now require length calculation.

Fortunately the documentation of [`std::ffi::CStr`] already included tons of warnings about future
changes, so we could assume users not relying on these performance characteristics in code.

[`std::ffi::CStr`]: https://doc.rust-lang.org/1.21.0/std/ffi/struct.CStr.html

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

An implementation of such change is available as the [`thin_cstr`] crate, and the source code is
available at <https://github.com/kennytm/thin_cstr>.

The change only affects the unsized `CStr` type. The owned `CString` type will not be modified.

[`thin_cstr`]: https://crates.io/crates/thin_cstr

## Drawbacks
[drawbacks]: #drawbacks

Assuming the C string has length *n*,

| Function | Before | After |
|:---------|:------:|:-----:|
| `from_ptr` | O(n) | O(1) |
| `from_bytes_with_nul` | O(n) | **O(n)** |
| `from_bytes_with_nul_unchecked` | O(1) | O(1) |
| `as_ptr` | O(1) | O(1) |
| `to_bytes` | O(1) | **O(n)** |
| `to_bytes_with_nul` | O(1) | **O(n)** |
| `to_str` | O(n) | O(n) |
| `to_string_lossy` | O(n) | O(n) |
| `into_c_string` | O(1) | **O(n)** |

Here, *only* `CStr::from_ptr` has become a zero-cost function, all other methods either still have
the same cost or become even slower. One particular issue is `CStr::into_c_string`, which was
stabilized in 1.20 but without the performance warning.

In `rustc` alone, most use of `CStr` will immediately convert it to a byte-slice or string, which
gives no performance advantage or disadvantage. Even worse, if we create the `&CStr` via
`CStr::from_bytes_with_nul`, the length calculation cost will be doubled.

```rust
let s = CStr::from_ptr(last_error).to_bytes();
```

## Rationale and alternatives
[alternatives]: #alternatives

The main rationale of this RFC is that `*CStr` being fat was considered a bug. An obvious
alternative is "not do this", accepting a fat `*CStr` as a feature. In this case, we would modify
the documentation and get rid of all mentions of potential performance changes.

We currently use extern type as this is the only way to get a thin DST. Extern types currently
implements none of the standard auto traits (`Send`, `Sync`, `Freeze`, `UnwindSafe`,
`RefUnwindSafe`), while a `[c_char]` slice implements all of them. Currently `Freeze` cannot be
manually implemented as it is [private in libcore][a]. Furthermore, it means whenever a new
auto-trait is introduced (probably by third-party), it will need to be manually implemented for
`CStr`. If this semantics of extern type cannot be modified, we may need to consider reviving the
custom DST RFC ([RFC 1524]) for more control.

[RFC 1524]: https://github.com/rust-lang/rfcs/pull/1524
[a]: https://github.com/rust-lang/rust/issues/43467#issuecomment-344955343

## Unresolved questions
[unresolved]: #unresolved-questions

How to make the thin `CStr` implement `Freeze`.
