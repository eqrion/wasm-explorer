package main

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/url"
	"os"
	"sync"
	"time"
)

// Entry represents a stored payload with timestamp
type Entry struct {
	Data      []byte
	Timestamp time.Time
}

// In-memory store for payloads
var (
	store   = make(map[string]*Entry)
	mutex   = &sync.RWMutex{}
	timeout = 30 * time.Minute // Configurable timeout duration
)

func main() {
	// Start cleanup goroutine
	go cleanupExpiredEntries()

	// Set up routes
	http.HandleFunc("/proxy", proxyHandler)
	http.HandleFunc("/store", storeHandler)
	http.HandleFunc("/retrieve", retrieveHandler)

	// Start server
	port := os.Getenv("PORT")
	if port == "" {
		port = ":8081" // Default port
	}
	fmt.Printf("Server starting on port %s\n", port)
	log.Fatal(http.ListenAndServe(port, nil))
}

func proxyHandler(w http.ResponseWriter, r *http.Request) {
	// Get target URL from query parameter
	targetURL := r.URL.Query().Get("url")
	if targetURL == "" {
		http.Error(w, "Missing 'url' query parameter", http.StatusBadRequest)
		return
	}

	// Validate URL
	_, err := url.Parse(targetURL)
	if err != nil {
		http.Error(w, "Invalid URL", http.StatusBadRequest)
		return
	}

	// Create request to target URL
	req, err := http.NewRequest(r.Method, targetURL, r.Body)
	if err != nil {
		http.Error(w, "Failed to create request", http.StatusInternalServerError)
		return
	}

	// Copy relevant headers (excluding host-specific ones)
	for key, values := range r.Header {
		if key != "Host" && key != "Origin" && key != "Referer" {
			for _, value := range values {
				req.Header.Add(key, value)
			}
		}
	}

	// Make the request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		http.Error(w, "Failed to fetch URL", http.StatusBadGateway)
		return
	}
	defer resp.Body.Close()

	// Copy response headers
	for key, values := range resp.Header {
		for _, value := range values {
			w.Header().Add(key, value)
		}
	}

	// Set CORS headers to allow browser access
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "*")

	// Copy status code and body
	w.WriteHeader(resp.StatusCode)
	io.Copy(w, resp.Body)
}

func storeHandler(w http.ResponseWriter, r *http.Request) {
	// Set CORS headers
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "*")

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read the payload
	payload, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Failed to read payload", http.StatusBadRequest)
		return
	}

	// Generate a unique short key
	keyBytes := make([]byte, 8)
	_, err = rand.Read(keyBytes)
	if err != nil {
		http.Error(w, "Failed to generate key", http.StatusInternalServerError)
		return
	}
	key := hex.EncodeToString(keyBytes)

	// Store the payload with timestamp
	entry := &Entry{
		Data:      payload,
		Timestamp: time.Now(),
	}
	mutex.Lock()
	store[key] = entry
	mutex.Unlock()

	// Return the key
	w.Header().Set("Content-Type", "text/plain")
	fmt.Fprintf(w, key)
}

func retrieveHandler(w http.ResponseWriter, r *http.Request) {
	// Set CORS headers
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "*")

	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Extract key from URL path
	key := r.URL.Path[len("/retrieve/"):]
	if key == "" {
		http.Error(w, "Missing key", http.StatusBadRequest)
		return
	}

	// Retrieve the payload
	mutex.RLock()
	entry, exists := store[key]
	mutex.RUnlock()

	if !exists {
		http.Error(w, "Key not found", http.StatusNotFound)
		return
	}

	// Check if entry has expired
	if time.Since(entry.Timestamp) > timeout {
		// Remove expired entry
		mutex.Lock()
		delete(store, key)
		mutex.Unlock()
		http.Error(w, "Key not found", http.StatusNotFound)
		return
	}

	// Return the payload
	w.Header().Set("Content-Type", "application/octet-stream")
	w.Write(entry.Data)
}

// cleanupExpiredEntries runs periodically to remove expired entries
func cleanupExpiredEntries() {
	ticker := time.NewTicker(1 * time.Minute) // Cleanup every minute
	defer ticker.Stop()

	for range ticker.C {
		now := time.Now()
		mutex.Lock()
		for key, entry := range store {
			if now.Sub(entry.Timestamp) > timeout {
				delete(store, key)
				log.Printf("Cleaned up expired entry: %s", key)
			}
		}
		mutex.Unlock()
	}
}
