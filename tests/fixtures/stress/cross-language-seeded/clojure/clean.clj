;; Clojure clean baseline — no composition breaks
;; All functions have matching return types at call sites.

(defn source-value []
  42)

(defn aggregate []
  (let [v (source-value)]
    v))