(defmacro unless [test body]
  `(if (not ~test) ~body))

(defn check [x]
  (unless (nil? x) (println "got" x)))

(defn main []
  (check 42)
  (check nil))