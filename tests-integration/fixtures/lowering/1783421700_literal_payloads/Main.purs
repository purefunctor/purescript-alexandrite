module Main where

string = "hello"

rawString = """hello"""

char = '\n'

charDoubleQuote = '\"'

charDoubleQuoteBinder '\"' = true
charDoubleQuoteBinder _ = false

charUnicode = '\x2603'

charMalformedNamed = '\n2'

charMalformedRaw = 'ab'

integer = 0x2a

number = 1.25
