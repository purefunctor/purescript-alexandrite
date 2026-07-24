module Main where

infixl 6 add as +

add :: Int -> Int -> Int
add left right = left

use :: Int -> String -> String
use integer string = string

checked :: Int -> Int
checked = _ + 1

inferred = _ + 1

ordered :: Int -> String -> String
ordered = use _ _

nested = _ (_ 1)

access = _.value

update = _ { first = _, second = _ }

conditional = if _ then _ else _
