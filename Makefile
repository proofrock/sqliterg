.PHONY: test

profile:
	bash profiler/stress_sqliterg.sh
	bash profiler/stress_ws4sqlite.sh

test:
	cd testing; go test -v -timeout 6m
