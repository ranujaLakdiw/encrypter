;; Stand-in for the import module `__wbindgen_externref_xform__`.
;;
;; wasm-bindgen 0.2.130 leaves this one import unresolved when its externref
;; transform is off:
;;   (import "__wbindgen_externref_xform__" "__wbindgen_externref_statics_end" (func (result i32)))
;; It is the first JS-heap index that is NOT one of the reserved slots
;; (0..1023 stack area, 1024 undefined, 1025 null, 1026 true, 1027 false).
;; With the transform ON, wasm-bindgen replaced every call with `i32.const 1028`
;; for this crate (observed by diffing the two builds), and the non-externref
;; glue hard-codes the same number in `dropObject` (`if (idx < 1028) return;`).
;; build-fallback.sh checks that the glue still says 1028 and stops if not.
(module
 (func (export "__wbindgen_externref_statics_end") (result i32)
  (i32.const 1028)
 )
)
