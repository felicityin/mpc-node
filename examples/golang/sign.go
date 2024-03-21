package main

/*
#cgo LDFLAGS: -L./lib -lctss
#include "./lib/ctss.h"
*/
import "C"
import (
	"fmt"
	"log"
	"os"
	"strconv"
)

func main() {
	fmt.Println("input index:")
	var indexStr string
	_, err := fmt.Scanln(&indexStr)
	if err != nil {
		log.Fatalln(err)
	}

	index, err := strconv.Atoi(indexStr)
	if err != nil {
		log.Fatalln(err)
	}

	localShare, err := os.ReadFile(fmt.Sprintf("local-share%d.json", index))
	if err != nil {
		log.Fatalln(err)
	}

	signature := C.sign(
		C.CString("http://localhost:8000/"),
		C.CString("default-keygen"),
		C.CString(string(localShare)),
		C.CString("1,2"),
		C.CString("hello"),
	)

	fmt.Println(C.GoString(signature))
}
