;; Clojure source for lifecycle fact extraction testing.
;;
;; Expected lifecycle facts:
;; - writes: binding assignments
;; - retries: loop/recur patterns
;; - resources: with-open usage
;; - exit paths: return values from defn body

(ns lifecycle-facts.core
  (:require [clojure.java.io :as io]))

(defn read-file
  "Read a file with with-open."
  [path]
  (with-open [r (io/reader path)]
    (doall (line-seq r))))

(defn write-file
  "Write to a file."
  [path content]
  (with-open [w (io/writer path)]
    (.write w content)))

(defn retry-operation
  "Retry pattern with loop/recur."
  [url max-retries]
  (loop [attempt 0
         last-error nil]
    (if (< attempt max-retries)
      (try
        (let [result (perform-request url)]
          result)
        (catch Exception e
          (recur (inc attempt) e)))
      false)))

(defn perform-request
  [url]
  true)