package main

// This file exists so that `go run .` will work in this
// directory and gives us something to refer to.
//
// This file will be automatically generated when bundles
// are created so it is never actually used; it exists
// solely for test purposes.

import (
	"os"
	"time"
)

type FileInfo = os.FileInfo

var fs = &EmbeddedFileSystem{
	assets: AssetMap {
		"/": &DirInfo {
			name: "/",	
			modTime: time.Date(2018, 5, 24, 2, 10, 23, 77500328, time.UTC),
		},
		"/index.html": &AssetFile {
			name: "index.html",	
			modTime: time.Date(2018, 5, 24, 2, 10, 23, 77500328, time.UTC),
			content: []byte("This is an index page"),
			size: 21,
		},
	},
};

func init () {
	fs.assets["/"].(*DirInfo).entries = []FileInfo{
		fs.assets["/index.html"].(FileInfo),
	}
}
