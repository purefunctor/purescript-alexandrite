module Main where

integer :: forall a. Int -> a -> a
integer (-1) value = value
integer _ value = value

number :: forall a. Number -> a -> a
number (-1.5) value = value
number _ value = value

string :: forall a. String -> a -> a
string "life" value = value
string _ value = value

character :: forall a. Char -> a -> a
character 'a' value = value
character _ value = value

boolean :: forall a. Boolean -> a -> a
boolean true value = value
boolean false value = value
