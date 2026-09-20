CXX      ?= g++
CXXFLAGS ?= -std=c++17 -O2 -Wall -Wextra
CPPFLAGS += -Ivendor

validate: validate.cpp
	$(CXX) $(CXXFLAGS) $(CPPFLAGS) -o $@ $<

test: validate
	@./run_fixtures.sh

clean:
	rm -f validate

.PHONY: test clean