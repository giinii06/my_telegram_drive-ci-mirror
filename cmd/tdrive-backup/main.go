// Command tdrive-backup archives a local folder into a TDrive host one file at
// a time.
//
// The agent runs on the machine that holds the files (the Windows backup
// installer points it at the TDrive host's Tailscale address). For each file it
// uploads through the host's web API, waits for the Telegram upload to finish,
// checks the host lists the file with the same byte count, and only then
// removes the local original. The Telegram copy is never deleted: dropping the
// original is the point, and the archive has to outlive it.
//
// State lives in config.json plus journal.json under the user's config
// directory. A restart resumes from the journal, so a file that is already
// archived is not uploaded twice.
package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"os/signal"
	"path"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

var (
	buildVersion = "dev"
	buildCommit  = "unknown"
)

const (
	defaultIntervalSeconds = 30
	defaultSettleSeconds   = 10
	defaultUploadTimeout   = 6 * time.Hour
	defaultRemoteFolder    = "/Backup"
)

type Config struct {
	Host                 string `json:"host"`
	Token                string `json:"token"`
	Folder               string `json:"folder"`
	Remote               string `json:"remote"`
	KeepOriginal         bool   `json:"keep_original"`
	IntervalSeconds      int    `json:"interval_seconds"`
	SettleSeconds        int    `json:"settle_seconds"`
	UploadTimeoutSeconds int    `json:"upload_timeout_seconds"`

	Path     string `json:"-"`
	LockPath string `json:"-"`
	LogPath  string `json:"-"`
}

func defaultConfigDir() (string, error) {
	base, err := os.UserConfigDir()
	if err != nil {
		return "", fmt.Errorf("locate config directory: %w", err)
	}
	return filepath.Join(base, "TDriveBackup"), nil
}

// configPaths resolves where the agent keeps its three files. An explicit
// --config overrides the config file only; the journal and lock stay beside it.
func configPaths(override string) (configPath, dir string, err error) {
	dir, err = defaultConfigDir()
	if err != nil {
		return "", "", err
	}
	configPath = filepath.Join(dir, "config.json")
	if override != "" {
		absolute, err := filepath.Abs(override)
		if err != nil {
			return "", "", err
		}
		configPath = absolute
		dir = filepath.Dir(absolute)
	}
	return configPath, dir, nil
}

func loadConfig(configPath, dir string) (Config, error) {
	body, err := os.ReadFile(configPath)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return Config{}, fmt.Errorf("no config at %s; run: tdrive-backup setup --host <tailscale-host> --token <token> --folder <path>", configPath)
		}
		return Config{}, err
	}
	var cfg Config
	if err := json.Unmarshal(body, &cfg); err != nil {
		return Config{}, fmt.Errorf("parse %s: %w", configPath, err)
	}
	cfg.Path = configPath
	cfg.LockPath = filepath.Join(dir, "agent.lock")
	cfg.LogPath = filepath.Join(dir, "backup.log")
	cfg.normalize()
	return cfg, nil
}

func (c *Config) normalize() {
	if strings.TrimSpace(c.Remote) == "" {
		c.Remote = defaultRemoteFolder
	}
	c.Remote = "/" + strings.TrimPrefix(path.Clean("/"+c.Remote), "/")
	if c.IntervalSeconds <= 0 {
		c.IntervalSeconds = defaultIntervalSeconds
	}
	if c.SettleSeconds < 0 {
		c.SettleSeconds = defaultSettleSeconds
	}
}

func (c Config) interval() time.Duration {
	if c.IntervalSeconds <= 0 {
		return defaultIntervalSeconds * time.Second
	}
	return time.Duration(c.IntervalSeconds) * time.Second
}

func (c Config) settle() time.Duration {
	return time.Duration(c.SettleSeconds) * time.Second
}

func (c Config) uploadTimeout() time.Duration {
	if c.UploadTimeoutSeconds <= 0 {
		return defaultUploadTimeout
	}
	return time.Duration(c.UploadTimeoutSeconds) * time.Second
}

// validate runs once at startup so a typo in the installer-supplied config
// fails with one clear message instead of a retry loop.
func (c *Config) validate() error {
	if _, err := newAPIClient(c.Host, c.Token); err != nil {
		return err
	}
	if strings.TrimSpace(c.Folder) == "" {
		return errors.New("backup folder is required")
	}
	absolute, err := filepath.Abs(c.Folder)
	if err != nil {
		return err
	}
	c.Folder = absolute
	if err := folderExists(c.Folder); err != nil {
		return fmt.Errorf("backup folder: %w", err)
	}
	if c.Remote == "" || !strings.HasPrefix(c.Remote, "/") {
		return fmt.Errorf("remote folder %q must start with /", c.Remote)
	}
	if c.IntervalSeconds < 5 {
		return errors.New("interval_seconds must be at least 5")
	}
	return nil
}

func (c Config) save() error {
	if err := os.MkdirAll(filepath.Dir(c.Path), 0o700); err != nil {
		return err
	}
	body, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return err
	}
	if err := os.WriteFile(c.Path, append(body, '\n'), 0o600); err != nil {
		return err
	}
	return nil
}

func main() {
	os.Exit(run(os.Args[1:]))
}

func run(args []string) int {
	if len(args) == 0 {
		printUsage(os.Stderr)
		return 2
	}
	switch args[0] {
	case "run", "once", "status", "setup":
		return runCommand(args[0], args[1:])
	case "version":
		fmt.Printf("tdrive-backup %s (%s)\n", buildVersion, buildCommit)
		return 0
	case "help", "-h", "--help":
		printUsage(os.Stdout)
		return 0
	default:
		fmt.Fprintf(os.Stderr, "unknown command %q\n\n", args[0])
		printUsage(os.Stderr)
		return 2
	}
}

func runCommand(command string, args []string) int {
	flags := flag.NewFlagSet(command, flag.ContinueOnError)
	flags.SetOutput(os.Stderr)
	configFlag := flags.String("config", "", "config file path")
	host := flags.String("host", "", "TDrive host, for example 100.101.102.103:8080")
	token := flags.String("token", "", "TDrive web access token")
	folder := flags.String("folder", "", "local folder to archive")
	remote := flags.String("remote", defaultRemoteFolder, "remote folder for archived files")
	keep := flags.Bool("keep", false, "keep the local original after a verified upload")
	interval := flags.Int("interval", defaultIntervalSeconds, "seconds between folder scans")
	if err := flags.Parse(args); err != nil {
		return 2
	}

	configPath, dir, err := configPaths(*configFlag)
	if err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}

	if command == "setup" {
		cfg := Config{
			Host:                 strings.TrimSpace(*host),
			Token:                strings.TrimSpace(*token),
			Folder:               strings.TrimSpace(*folder),
			Remote:               *remote,
			KeepOriginal:         *keep,
			IntervalSeconds:      *interval,
			SettleSeconds:        defaultSettleSeconds,
			UploadTimeoutSeconds: int(defaultUploadTimeout.Seconds()),
			Path:                 configPath,
			LockPath:             filepath.Join(dir, "agent.lock"),
			LogPath:              filepath.Join(dir, "backup.log"),
		}
		if cfg.Token == "" {
			fmt.Fprintln(os.Stderr, "warning: no token set; only hosts running with --no-auth accept this")
		}
		if err := os.MkdirAll(cfg.Folder, 0o755); err != nil && cfg.Folder != "" {
			fmt.Fprintf(os.Stderr, "tdrive-backup: create backup folder: %v\n", err)
			return 1
		}
		cfg.normalize()
		if err := cfg.validate(); err != nil {
			fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
			return 1
		}
		if err := cfg.save(); err != nil {
			fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
			return 1
		}
		fmt.Printf("config: %s\nhost: %s\nfolder: %s\nremote: %s\nkeep original: %v\n",
			cfg.Path, cfg.Host, cfg.Folder, cfg.Remote, cfg.KeepOriginal)
		return 0
	}

	cfg, err := loadConfig(configPath, dir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}
	if err := cfg.validate(); err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}
	logger, closeLog, err := openLog(cfg.LogPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}
	defer closeLog()

	client, err := newAPIClient(cfg.Host, cfg.Token)
	if err != nil {
		logger.Printf("error: %v", err)
		return 1
	}
	store, err := loadJournal(filepath.Join(dir, "journal.json"))
	if err != nil {
		logger.Printf("error: %v", err)
		return 1
	}

	if command == "status" {
		return printStatus(cfg, store)
	}

	lock, err := acquireLock(cfg.LockPath)
	if err != nil {
		logger.Printf("error: %v", err)
		return 1
	}
	defer lock.release()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	backup := newAgent(cfg, client, store, logger.Printf)

	if command == "once" {
		if err := client.Ping(ctx); err != nil {
			logger.Printf("error: host %s: %v", cfg.Host, err)
			return 1
		}
		processed, err := backup.pass(ctx)
		if err != nil {
			logger.Printf("error: %v", err)
			return 1
		}
		if !processed {
			logger.Printf("nothing ready to archive")
		}
		return 0
	}

	logger.Printf("tdrive-backup %s (%s) starting", buildVersion, buildCommit)
	if err := client.Ping(ctx); err != nil {
		logger.Printf("host not reachable yet: %v", err)
	}
	return codeOf(backup.run(ctx))
}

func codeOf(err error) int {
	if err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}
	return 0
}

func printStatus(cfg Config, store *journalStore) int {
	files, err := scanFolder(cfg.Folder, nil)
	if err != nil {
		fmt.Fprintf(os.Stderr, "tdrive-backup: %v\n", err)
		return 1
	}
	pending := 0
	for _, file := range files {
		if _, done := store.done(file.rel, file.size, file.stamp.mod); !done {
			pending++
		}
	}
	fmt.Printf("host: %s\nfolder: %s\nremote: %s\nkeep original: %v\narchived: %d\npending: %d\n",
		cfg.Host, cfg.Folder, cfg.Remote, cfg.KeepOriginal, len(store.data.Uploaded), pending)
	return 0
}

// openLog appends to the agent's log and mirrors it to stderr, which is hidden
// when the logon task starts the agent but visible in a manual run.
func openLog(path string) (*log.Logger, func(), error) {
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return nil, nil, err
	}
	file, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o600)
	if err != nil {
		return nil, nil, err
	}
	writer := io.MultiWriter(os.Stderr, file)
	logger := log.New(writer, "", log.LstdFlags)
	return logger, func() { _ = file.Close() }, nil
}

func printUsage(out io.Writer) {
	fmt.Fprint(out, `tdrive-backup archives a folder into a TDrive host one file at a time.

usage:
  tdrive-backup setup --host <tailscale-host> --token <token> --folder <path> [--remote /Backup] [--keep] [--interval 30]
  tdrive-backup run [--config <path>]
  tdrive-backup once [--config <path>]
  tdrive-backup status [--config <path>]
  tdrive-backup version

Each file is uploaded through the host, verified against the host's listing, and
then removed from the local folder unless --keep is set. The Telegram copy is
never deleted.
`)
}
