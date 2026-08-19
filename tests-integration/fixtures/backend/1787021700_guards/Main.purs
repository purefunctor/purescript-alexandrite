module Main where

data Choice a = Empty | One a

booleanGuard :: Boolean -> Int
booleanGuard value
  | value = 1
  | true = 0

patternGuard :: Choice Int -> Int
patternGuard choice
  | One value <- choice = value
  | true = 0
