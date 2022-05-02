if exists("b:current_syntax")
  finish
endif

" Numbers
syn match filamentConstant "\v<[0-9]+('d[0-9]+)?>"

" Type binding and interval annotations
syn match filamentOperator "\v\@within|\@exact" nextgroup=filamentInterval skipwhite
syn region filamentInterval start="\v\(" end="\v\)" contained contains=filamentType,filamentConstant

" Components
syn keyword filamentKeyword component nextgroup=filamentCompName skipwhite
syn match filamentCompName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=filamentAbsVar skipwhite

syn region filamentAbsVar start="\v\<" end="\v\>" contains=filamentType
syn match filamentType '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained

" Comments
syn match filamentComment "\v//.*$"
syn region filamentComment start=/\v\/\*/ skip=/\v\\./ end=/\v\*\//

" Cells
syn keyword filamentKeyword cells nextgroup=filamentCells skipwhite
syn region filamentCells start="\v\{" end="\v\}" contained skipwhite skipnl contains=filamentCellName

syn match filamentCellName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=filamentColon skipwhite
syn match filamentColon ':' contained nextgroup=filamentType contained skipwhite
hi link filamentCellName filamentCompName

" When statements
syn match filamentKeyword 'when'
syn match filamentOperator '@rises'
syn match filamentKeyword 'at' nextgroup=filamentType skipwhite
syn match filamentKeyword 'time' nextgroup=filamentType skipwhite

" Ports
syn match filamentDot '\.' nextgroup=filamentPort
syn match filamentPort '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained

" Other keywords
syn keyword filamentSpecialKeywords extern

hi link filamentRises filamentOperator
hi link filamentSpecialKeywords Directory
hi link filamentPort Directory
hi link filamentKeyword Keyword
hi link filamentControl Special
hi link filamentCompName Include
hi link filamentType Type
hi link filamentComment Comment
hi link filamentConstant Constant
hi link filamentOperator Operator

let b:current_syntax = "filament"
