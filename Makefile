build: test.c muse.h
	mkdir -p build/
	gcc -o build/test -ggdb -Wall -Wextra test.c -lraylib
