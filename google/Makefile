all: out/cgrep rm-temp

TEMP:=$(shell mktemp -d)

out/cgrep: Makefile Dockerfile
	mkdir -p out
	docker run -v $(TEMP):/out $(shell docker build --quiet .) sh -c 'cp bin/* /out'
	cp -ar $(TEMP)/* out/

rm-temp:
	$(RM) $(TEMP)/*
	$(RM) -d $(TEMP)

clean:
	rm -rf out
