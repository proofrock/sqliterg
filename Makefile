.PHONY: test

profile:
	bash profiler/stress_sqliterg.sh
	bash profiler/stress_ws4sqlite.sh