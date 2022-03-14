#!/bin/zsh

ott -i types.ott -o types.tex -tex_show_meta false
tectonic -X compile types.tex
