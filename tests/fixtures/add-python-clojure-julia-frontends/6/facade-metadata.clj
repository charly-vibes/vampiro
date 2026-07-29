;; Clojure namespace facade metadata fixture.
;;
;; Expected facade declarations:
;; - :rename aliased imports
;; - :refer :all wildcard re-exports

(ns facade-metadata.core
  (:require [clojure.string :as str]
            [clojure.set :refer [union intersection]]
            [clojure.java.io :refer :all])
  (:use [clojure.walk :only [keywordize-strings]]))