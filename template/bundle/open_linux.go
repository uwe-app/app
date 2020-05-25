// +build linux

package main

func withOptions(opts *Options) {
	opts.Launch.Name = "xdg-open"
}
