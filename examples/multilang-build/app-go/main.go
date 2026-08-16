// Go application — part of multi-language build example.
package main

/*
#include <stdlib.h>
// Link against the C utility library
void to_upper(char *s);
void str_reverse(char *s);
int count_char(const char *s, char c);
*/
import "C"
import "fmt"

func main() {
	// Use C library functions.
	str := C.CString("hello multilang")
	defer C.free(unsafe.Pointer(str))

	C.to_upper(str)
	fmt.Printf("Uppercase: %s\n", C.GoString(str))

	C.str_reverse(str)
	fmt.Printf("Reversed: %s\n", C.GoString(str))

	count := C.count_char(C.CString("hello world"), 'l')
	fmt.Printf("Count of 'l': %d\n", count)

	fmt.Println("Multi-language app running!")
}
