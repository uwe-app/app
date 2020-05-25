package main

import (
	"fmt"
	"log"
	"net"
	"net/http"
	"os/exec"
	"strconv"
)

type Command struct {
	Name string
	Args []string
}

type Options struct {
	Host string
	Port int
	Launch *Command
}

func serve(ln net.Listener, opts *Options) {
	fs := http.Dir(".")
	fileServer := http.FileServer(fs)
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
		fileServer.ServeHTTP(w, r)
	})
	if err := http.Serve(ln, nil); err != nil {
		log.Fatal(err)
	}
}

func launch(url string, cmd *Command) {
	args := append(cmd.Args, url)
	ps := exec.Command(cmd.Name, args...)
	ps.Run()
}

func main() {
	opts := &Options{
		Host: "localhost",
		Port: 0,
		Launch: &Command{},
	}
	withOptions(opts)

	targetPort := strconv.Itoa(opts.Port)
	bind := net.JoinHostPort(opts.Host, targetPort)
	ln, err := net.Listen("tcp", bind)
	if err != nil {
		log.Fatal(err)
	}
	defer ln.Close()

	go serve(ln, opts)

	_, port, err := net.SplitHostPort(ln.Addr().String())
	if err != nil {
		log.Fatal(err)	
	}

	location := net.JoinHostPort(opts.Host, port)
	url := fmt.Sprintf("http://%s", location)

	log.Printf("Web server running at %s:%s", opts.Host, port)
	log.Println("Press Ctrl+c to exit the program")

	launch(url, opts.Launch)

	for {}
}
