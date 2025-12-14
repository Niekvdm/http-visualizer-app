// Package config provides configuration loading from environment variables.
package config

import (
	"os"
	"strconv"
)

const (
	// DefaultPort is the default HTTP server port.
	DefaultPort = 3000
)

// Config holds the application configuration.
type Config struct {
	Port int
}

// Load loads configuration from environment variables.
func Load() *Config {
	return &Config{
		Port: getEnvInt("PORT", DefaultPort),
	}
}

func getEnvInt(key string, defaultVal int) int {
	if val := os.Getenv(key); val != "" {
		if i, err := strconv.Atoi(val); err == nil {
			return i
		}
	}
	return defaultVal
}
