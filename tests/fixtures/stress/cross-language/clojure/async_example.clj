;; Core async patterns — channels, go blocks, pipeline

(ns async-example
  (:require [clojure.core.async :as async
             :refer [chan go go-loop >! <! >!! <!! close! timeout]]))

(defn producer [ch n]
  (go-loop [i 0]
    (when (< i n)
      (>! ch i)
      (recur (inc i)))
    (close! ch)))

(defn consumer [ch label]
  (go-loop []
    (when-let [val (<! ch)]
      (println label "got:" val)
      (recur))))

(defn pipeline [n]
  (let [in (chan)
        out (chan)]
    (go
      (loop [i 0]
        (when-let [val (<! in)]
          (>! out (* val 2))
          (recur (inc i)))))
    (producer in n)
    (consumer out "doubled")
    out))

(defn -main [& args]
  (let [n (if (seq args) (Integer/parseInt (first args)) 10)]
    (println "Starting pipeline with" n "items")
    (let [ch (pipeline n)]
      (async/<!! (async/into [] (async/take n ch))))))