// +build darwin

package main

func withOptions(opts *Options) {
	opts.Launch.Name = "open"
}
