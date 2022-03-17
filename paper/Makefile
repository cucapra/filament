TARGETS := filament
EXTRADEPS := $(wildcard ./*.sty) acmart.cls semantics/types.tex $(wildcard fig/*.pdf) $(wildcard fig/*.pdf)
EXTRACLEAN := semantics/types.tex
include Makefile.include

semantics/types.tex: semantics/types.ott
	ott -i semantics/types.ott -o semantics/types.tex -tex_wrap false

semantics/types.pdf: semantics/types.ott
	ott -i semantics/types.ott -o semantics/types.tex -tex_show_meta false
	tectonic -X compile semantics/types.tex

sources.zip: $(TARGETS:=.tex) $(EXTRADEPS)
	zip -r sources.zip bib fig $(TARGETS:=.tex) acmart.cls pervasives.sty
