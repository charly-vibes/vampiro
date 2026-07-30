;; Clojure data-flow edge exercise
;; Tests that per-slot edges are emitted for call arguments with known shapes.
;; The call (add 42 3) should produce 2 slot edges (slots 0 and 1)
;; with expression nodes for the integer literals.

(defn add [a b]
  (+ a b))

(defn main []
  (add 42 3))