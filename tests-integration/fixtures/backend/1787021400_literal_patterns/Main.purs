module Main where

integer :: Int -> Boolean
integer 0 = true
integer _ = false

number :: Number -> Boolean
number 1.5 = true
number _ = false

character :: Char -> Boolean
character 'a' = true
character _ = false

string :: String -> Boolean
string "alexandrite" = true
string _ = false

boolean :: Boolean -> Boolean
boolean true = true
boolean false = false
