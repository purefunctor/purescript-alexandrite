module Main where

string :: String
string = "line\n\"quote\"\\end"

rawString :: String
rawString = """line\n"quoted"\\end"""

character :: Char
character = 'λ'

characterNewline :: Char
characterNewline = '\n'

characterCarriageReturn :: Char
characterCarriageReturn = '\r'

characterTab :: Char
characterTab = '\t'

characterQuote :: Char
characterQuote = '\''

characterBackslash :: Char
characterBackslash = '\\'

characterNull :: Char
characterNull = '\0'

characterHexadecimal :: Char
characterHexadecimal = '\x2603'

characterDelete :: Char
characterDelete = '\x7f'

characterControl :: Char
characterControl = '\x85'
