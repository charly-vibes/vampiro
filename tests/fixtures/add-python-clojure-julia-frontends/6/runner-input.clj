;; Clojure source for runner-input extraction testing.
;;
;; Expected runner-input fields:
;; - version: "0.1.0"
;; - source_file: "runner-input.clj"
;; - tagged_fns: defn declarations with params and return types
;; - serializable_values: parameters with primitive types
;; - generator_refs: lazy-seq generators

(ns runner-input.core)

(defn add
  "Add two numbers."
  [a b]
  (+ a b))

(defn greet
  "Greet someone."
  [name]
  (str "Hello, " name "!"))

(defn process
  "Process a collection."
  [items]
  (when (seq items)
    (first items)))

(defn count-up-to
  "Generator function."
  [n]
  (lazy-seq (cons n (count-up-to (inc n)))))