package daemon

import (
	"bufio"
	"context"
	"net"
	"testing"
	"time"
)

func TestLoginContextClosesSocketOnCancel(t *testing.T) {
	clientConn, serverConn := net.Pipe()
	defer serverConn.Close()
	client := &Client{dial: func() (net.Conn, error) { return clientConn, nil }}

	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan error, 1)
	go func() {
		_, err := client.LoginContext(ctx, "+123", nil)
		done <- err
	}()

	if _, err := bufio.NewReader(serverConn).ReadString('\n'); err != nil {
		t.Fatalf("read login request: %v", err)
	}
	cancel()
	select {
	case err := <-done:
		if err == nil {
			t.Fatal("LoginContext returned nil after cancel")
		}
	case <-time.After(2 * time.Second):
		t.Fatal("LoginContext did not return after cancel")
	}
}
