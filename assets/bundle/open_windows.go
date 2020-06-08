// +build windows

package main

import (
	"os"
	"path/filepath"
)

func withOptions(opts *Options) {
	opts.Launch.Name = filepath.Join(os.Getenv("SYSTEMROOT"), "System32", "rundll32.exe")
	opts.Launch.Args = []string{"url.dll,FileProtocolHandler"}
}

