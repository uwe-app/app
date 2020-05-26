package main

import (
	"os"
	"time"
)

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
	fs.assets["/"].(*DirInfo).entries = []os.FileInfo{
		fs.assets["/index.html"].(os.FileInfo),
	}
}
