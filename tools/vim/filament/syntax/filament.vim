if exists("b:current_syntax")
  finish
endif

" Numbers
syn match filamentConstant "\v<[0-9]+('d[0-9]+)?>"

" Type binding and interval annotations
syn match filamentOperator "\v\@(exact)?" nextgroup=filamentInterval skipwhite
syn region filamentInterval start="\v\[" end="\v\]" contained contains=filamentType,filamentConstant

syn match filamentOperator "\v\@interface" nextgroup=filamentInterval skipwhite

" Components
syn match filamentKeyword "\vcomponent|new" nextgroup=filamentCompName skipwhite
syn match filamentCompName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=filamentAbsVar skipwhite

syn region filamentAbsVar start="\v\<" end="\v\>" contains=filamentType,filamentConstant
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
syn keyword filamentKeyword when import where fsm invoke

" Ports
syn match filamentDot '\.' nextgroup=filamentPort
syn match filamentPort '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained

" Strings
syn region filamentString start=/\v"/ skip=/\v\\./ end=/\v("|$)/
hi link filamentString String

" Other keywords
syn keyword filamentSpecialKeywords extern

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
