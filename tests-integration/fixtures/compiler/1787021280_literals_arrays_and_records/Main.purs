module Main where

integer :: Int
integer = 42

number :: Number
number = 1.5

character :: Char
character = 'a'

string :: String
string = "alexandrite"

boolean :: Boolean
boolean = true

array :: Array Int
array = [1, 2, 3]

record :: { integer :: Int, nested :: { value :: String } }
record = { integer: 42, nested: { value: "nested" } }

projection :: String
projection = record.nested.value
