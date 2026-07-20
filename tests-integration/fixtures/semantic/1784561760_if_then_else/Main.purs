module Main where

checked :: Boolean -> Int
checked condition = if condition then 1 else 2

inferred condition = if condition then "yes" else "no"
