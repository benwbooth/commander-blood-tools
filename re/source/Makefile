CC ?= cc
CFLAGS ?= -std=c89 -Wall -Wextra -pedantic -I. -Iinclude

OBJECTS = \
	bloodprg/object_heap.o \
	bloodprg/vm_records.o

.PHONY: check clean

check: $(OBJECTS)

clean:
	rm -f $(OBJECTS)
