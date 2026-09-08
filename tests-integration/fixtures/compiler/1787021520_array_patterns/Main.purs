module Main where

describe :: Array Int -> Int
describe [] = 0
describe [value] = value
describe [first, second] = first
describe _ = 3
