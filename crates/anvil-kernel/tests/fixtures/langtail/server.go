// LANGTAIL T1 fixture — representative Go source.
package server

import (
	"fmt"
	"net/http"

	mux "github.com/gorilla/mux"
)

// Server wraps an HTTP router.
type Server struct {
	Addr   string
	router *mux.Router
}

// Handler is the request-handling contract.
type Handler interface {
	Serve(w http.ResponseWriter, r *http.Request)
}

// New constructs a Server.
func New(addr string) *Server {
	return &Server{Addr: addr}
}

// Start runs the server.
func (s *Server) Start() error {
	return fmt.Errorf("not implemented")
}

func internalSetup() {}
