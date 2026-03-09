package openclaw

import (
	"fmt"
	"log"
	"os/exec"
	"runtime"
	"strings"
	"time"
)

// WSLManager handles WSL interactions: path translation, process management,
// gateway lifecycle, and lock file cleanup.
type WSLManager struct {
	distro    string // WSL distro name (e.g. "Ubuntu")
	available *bool
}

// NewWSLManager creates a WSL manager, auto-detecting the best distro.
func NewWSLManager() *WSLManager {
	m := &WSLManager{}
	m.distro = m.findDistro()
	return m
}

// IsAvailable checks if WSL is present on this system.
func (m *WSLManager) IsAvailable() bool {
	if m.available != nil {
		return *m.available
	}

	if runtime.GOOS != "windows" {
		f := false
		m.available = &f
		return false
	}

	cmd := exec.Command("wsl", "--status")
	err := cmd.Run()
	result := err == nil
	m.available = &result
	log.Printf("[wsl] available: %v", result)
	return result
}

// Distro returns the detected WSL distribution name.
func (m *WSLManager) Distro() string {
	return m.distro
}

// WindowsToWSLPath converts a Windows path to its WSL equivalent.
// Example: C:\Users\test\file.txt -> /mnt/c/Users/test/file.txt
func (m *WSLManager) WindowsToWSLPath(winPath string) string {
	path := strings.ReplaceAll(winPath, "\\", "/")
	if len(path) >= 2 && path[1] == ':' {
		drive := strings.ToLower(string(path[0]))
		rest := path[2:]
		return fmt.Sprintf("/mnt/%s%s", drive, rest)
	}
	return path
}

// WSLToWindowsPath converts a WSL path back to Windows format.
// Example: /mnt/c/Users/test/file.txt -> C:\Users\test\file.txt
func (m *WSLManager) WSLToWindowsPath(wslPath string) string {
	if strings.HasPrefix(wslPath, "/mnt/") && len(wslPath) >= 6 {
		drive := strings.ToUpper(string(wslPath[5]))
		rest := wslPath[6:]
		return strings.ReplaceAll(fmt.Sprintf("%s:%s", drive, rest), "/", "\\")
	}
	return wslPath
}

// RunCommand executes a command inside WSL.
func (m *WSLManager) RunCommand(command string, timeout time.Duration) (stdout, stderr string, exitCode int) {
	if m.distro == "" {
		return "", "no WSL distro found", -1
	}

	cmd := exec.Command("wsl", "-d", m.distro, "--", "bash", "-lc", command)

	done := make(chan error, 1)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf

	if err := cmd.Start(); err != nil {
		return "", fmt.Sprintf("failed to start: %v", err), -1
	}

	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				return outBuf.String(), errBuf.String(), exitErr.ExitCode()
			}
			return outBuf.String(), errBuf.String(), -1
		}
		return outBuf.String(), errBuf.String(), 0
	case <-time.After(timeout):
		cmd.Process.Kill()
		return "", fmt.Sprintf("command timed out after %s", timeout), -1
	}
}

// CleanStaleLockFiles removes stale OpenClaw gateway lock files in WSL.
// These get left behind when the gateway is killed without cleanup
// (common on WSL where systemd/openclaw gateway stop don't work).
func (m *WSLManager) CleanStaleLockFiles() error {
	if m.distro == "" {
		return fmt.Errorf("no WSL distro found")
	}

	cmd := exec.Command("wsl", "-d", m.distro, "--", "bash", "-c",
		"rm -f /tmp/openclaw-*/gateway.*.lock")
	cmd.Start()

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			log.Printf("[wsl] lock file cleanup failed: %v", err)
			return err
		}
		log.Printf("[wsl] cleaned stale lock files")
		return nil
	case <-time.After(5 * time.Second):
		cmd.Process.Kill()
		return fmt.Errorf("lock file cleanup timed out")
	}
}

// ForceKillOpenClaw kills ALL OpenClaw processes in WSL and removes lock files.
// This is the "big red button."
func (m *WSLManager) ForceKillOpenClaw() error {
	if m.distro == "" {
		return fmt.Errorf("no WSL distro found")
	}

	cmd := exec.Command("wsl", "-d", m.distro, "--", "bash", "-c",
		"pkill -f openclaw-gateway; pkill -f openclaw; rm -f /tmp/openclaw-*/gateway.*.lock; true")
	if err := cmd.Run(); err != nil {
		log.Printf("[wsl] force kill error: %v", err)
		return err
	}

	log.Printf("[wsl] force-killed all OpenClaw processes and removed lock files")
	return nil
}

// StartGateway spawns the OpenClaw gateway process in WSL.
// Returns the exec.Cmd so the caller can manage its lifecycle.
func (m *WSLManager) StartGateway() (*exec.Cmd, error) {
	if m.distro == "" {
		return nil, fmt.Errorf("no WSL distro found")
	}

	// Clean stale locks before starting
	m.CleanStaleLockFiles()

	cmd := exec.Command("wsl", "-d", m.distro, "-e", "bash", "-lc", "openclaw gateway")
	cmd.Stdout = log.Writer()
	cmd.Stderr = log.Writer()

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("failed to start OpenClaw gateway: %w", err)
	}

	log.Printf("[wsl] spawned OpenClaw gateway PID: %d", cmd.Process.Pid)
	return cmd, nil
}

func (m *WSLManager) findDistro() string {
	if runtime.GOOS != "windows" {
		return ""
	}

	cmd := exec.Command("wsl", "--list", "--quiet")
	out, err := cmd.Output()
	if err != nil {
		return ""
	}

	lines := strings.Split(string(out), "\n")
	var distros []string
	for _, line := range lines {
		// WSL output on Windows has null bytes
		cleaned := strings.ReplaceAll(strings.TrimSpace(line), "\x00", "")
		if cleaned != "" {
			distros = append(distros, cleaned)
		}
	}

	// Prefer Ubuntu
	for _, d := range distros {
		if strings.Contains(strings.ToLower(d), "ubuntu") {
			return d
		}
	}

	// Fall back to first non-docker distro
	for _, d := range distros {
		if !strings.Contains(strings.ToLower(d), "docker") {
			return d
		}
	}

	return ""
}
