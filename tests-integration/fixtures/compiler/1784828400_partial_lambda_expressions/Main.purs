module Main where

data Maybe a = Nothing | Just a

checked :: Partial => Maybe Int -> Int
checked = \(Just value) -> value

inferred = \(Just value) -> value
