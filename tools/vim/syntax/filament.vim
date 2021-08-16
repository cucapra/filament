if exists("b:current_syntax")
  finish
endif

" Numbers
syn match filamentConstant "\v<[0-9]+('d[0-9]+)?>"

" Type binding and interval annotations
syn match filamentOperator "\v\@within|\@exact" nextgroup=filamentInterval skipwhite
syn region filamentInterval start="\v\(" end="\v\)" contained contains=filamentType,filamentConstant

" Control statements
syn keyword filamentControl cells

" Other keywords
syn keyword filamentKeyword import cells wires control group prim extern

" Components
syn keyword filamentKeyword component nextgroup=filamentCompName skipwhite
syn match filamentCompName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=filamentAbsVar skipwhite

syn region filamentAbsVar start="\v\<" end="\v\>" contained contains=filamentType
syn match filamentType '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained

" Comments
syntax match filamentComment "\v//.*$"
syn region filamentComment start=/\v\/\*/ skip=/\v\\./ end=/\v\*\//

hi link filamentKeyword Keyword
hi link filamentControl Special
hi link filamentCompName Include
hi link filamentType Type
hi link filamentComment Comment
hi link filamentConstant Constant
hi link filamentOperator Operator

let b:current_syntax = "filament"
