;; Ring-compatible HTTP handler and middleware patterns

(ns http.server
  (:require [clojure.string :as str]))

;; --- Types / Records ---

(defrecord Request [method uri headers body])
(defrecord Response [status headers body])

(defn ok
  ([body] (->Response 200 {"content-type" "text/plain"} body))
  ([body content-type] (->Response 200 {"content-type" content-type} body)))

(defn not-found [msg]
  (->Response 404 {"content-type" "text/plain"} msg))

;; --- Middleware ---

(defn wrap-logging [handler]
  (fn [req]
    (println "→" (:method req) (:uri req))
    (let [resp (handler req)]
      (println "←" (:status resp))
      resp)))

(defn wrap-json [handler]
  (fn [req]
    (let [resp (handler req)]
      (if (= "application/json" (get-in req [:headers "accept"]))
        (assoc resp :headers {"content-type" "application/json"})
        resp))))

;; --- Router ---

(defn router [routes]
  (fn [req]
    (let [uri (:uri req)
          match (some (fn [[pattern handler]]
                       (when (re-find (re-pattern pattern) uri)
                         handler))
                     routes)]
      (if match
        (match req)
        (not-found (str "No route for " uri))))))

;; --- App ---

(defn home-handler [req]
  (ok "Welcome to the API"))

(defn user-handler [req]
  (let [user-id (some->> (:uri req)
                        (re-find #"/users/(\d+)")
                        second
                        Integer/parseInt)]
    (if user-id
      (ok (str "User " user-id))
      (not-found "Missing user ID"))))

(def app
  (-> (router [["^/$" home-handler]
               ["^/users/" user-handler]])
      wrap-logging
      wrap-json))