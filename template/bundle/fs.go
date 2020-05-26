package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path"
	"time"
)

type Asset interface {
	Name() string
}

type AssetMap map[string]Asset

type DirInfo struct {
	name string
	modTime time.Time
	entries []os.FileInfo
	pos int
}

type AssetFile struct {
	name string
	modTime time.Time
	content []byte
	size int64
	seekPos int64
}

// AssetFile
func (f *AssetFile) Read(p []byte) (n int, err error) {
	copied := copy(p, f.content)
	return copied, nil
}
func (f *AssetFile) Readdir(count int) ([]os.FileInfo, error) {
	return nil, fmt.Errorf("cannot readdir from file %s", f.name)
}
func (f *AssetFile) Seek(offset int64, whence int) (int64, error) {
	switch whence {
		case io.SeekStart:
			f.seekPos = 0 + offset
		case io.SeekCurrent:
			f.seekPos += offset
		case io.SeekEnd:
			f.seekPos = f.size + offset
		default:
			panic(fmt.Errorf("invalid whence value: %v", whence))
	}
	return f.seekPos, nil
}

func (f *AssetFile) Close() error									{ return nil }
func (f *AssetFile) Stat() (os.FileInfo, error)		{ return f, nil }
func (f *AssetFile) Name() string									{ return f.name }
func (f *AssetFile) Size() int64									{ return f.size }
func (f *AssetFile) Mode() os.FileMode						{ return 0444 }
func (f *AssetFile) ModTime() time.Time						{ return f.modTime }
func (f *AssetFile) IsDir() bool									{ return false }
func (f *AssetFile) Sys() interface{}							{ return nil }

// DirInfo
func (d *DirInfo) Read(p []byte) (n int, err error) {
	return 0, fmt.Errorf("cannot read from directory %s", d.name)
}
func (d *DirInfo) Close() error										{ return nil }
func (d *DirInfo) Stat() (os.FileInfo, error)			{ return d, nil }
func (d *DirInfo) Name() string										{ return d.name }
func (d *DirInfo) Size() int64										{ return 0 }
func (d *DirInfo) Mode() os.FileMode							{ return 0755 | os.ModeDir }
func (d *DirInfo) ModTime() time.Time							{ return d.modTime }
func (d *DirInfo) IsDir() bool										{ return true }
func (d *DirInfo) Sys() interface{}								{ return nil }

func (d *DirInfo) Seek(offset int64, whence int) (int64, error) {
	if offset == 0 && whence == io.SeekStart {
		d.pos = 0
		return 0, nil
	}
	return 0, fmt.Errorf("cannot seek in directory %s", d.name)
}

func (d *DirInfo) Readdir(count int) ([]os.FileInfo, error) {
	if d.pos >= len(d.entries) && count > 0 {
		return nil, io.EOF
	}
	if count <= 0 || count > len(d.entries)-d.pos {
		count = len(d.entries) - d.pos
	}
	e := d.entries[d.pos : d.pos+count]
	d.pos += count
	return e, nil
}

type EmbeddedFileSystem struct {
	assets AssetMap
}

func (fs *EmbeddedFileSystem) Open(pth string) (http.File, error) {
	asset := path.Clean("/" + pth)
	f, ok := fs.assets[asset]
	if !ok {
		return nil, &os.PathError{Op: "open", Path: asset, Err: os.ErrNotExist}
	}
	switch f := f.(type) {
		case *DirInfo:
			return f, nil
		case *AssetFile:
			return f, nil
		default:
			panic(fmt.Sprintf("unexpected type %T", f))
	}

	return nil, os.ErrNotExist
}
