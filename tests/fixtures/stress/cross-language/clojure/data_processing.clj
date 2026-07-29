;; Data processing with transducers, reducers, and lazy sequences

(ns data-processing
  (:require [clojure.java.io :as io]))

;; --- Record parsing ---

(defrecord Record [id name value tags])

(defn parse-line [line]
  (when-not (or (str/blank? line) (str/starts-with? line "#"))
    (let [parts (str/split line #",")]
      (when (>= (count parts) 3)
        (->Record (Integer/parseInt (nth parts 0))
                  (nth parts 1)
                  (Double/parseDouble (nth parts 2))
                  (vec (drop 3 parts)))))))

(defn parse-csv [text]
  (into [] (comp (remove #(or (str/blank? %) (str/starts-with? % "#")))
                 (map parse-line)
                 (remove nil?))
        (str/split-lines text)))

;; --- Aggregation pipeline ---

(defn aggregate-by [records key-fn]
  (reduce (fn [acc r]
            (let [k (key-fn r)]
              (update acc k (fnil + 0.0) (:value r))))
          {}
          records))

(defn filter-min [agg min-val]
  (into (sorted-map-by (fn [a b] (compare [(get agg b) b] [(get agg a) a])))
        (filter #(>= (val %) min-val))
        agg))

(defn report
  "Generate a sorted report from CSV text."
  [csv-text & {:keys [min-value] :or {min-value 0.0}}]
  (let [records (parse-csv csv-text)
        agg (aggregate-by records :name)]
    (filter-min agg min-value)))